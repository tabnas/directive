/* Copyright (c) 2021-2026 Richard Rodger and other contributors, MIT License */

//! Directive syntax plugin for the [tabnas](https://github.com/tabnas/parser)
//! parser engine.
//!
//! A *directive* is a token sequence — `@name` (open-only) or `add<1,2>`
//! (open + close) — that pushes into a dedicated rule and fires an action
//! to transform the parsed body. This crate is **not a parser of its own**:
//! it layers onto whatever host grammar supplies the standard `val` /
//! `list` / `map` / `pair` / `elem` rules, adds open/close tokens, modifies
//! those host rules, and installs one rule named after the directive.
//!
//! ```no_run
//! use tabnas::{Tabnas, Value};
//! use tabnas_directive::{apply, DirectiveOptions};
//!
//! let mut parser = Tabnas::new();
//! // ... install a host grammar supplying val/list/map/pair/elem ...
//! apply(
//!     &mut parser,
//!     DirectiveOptions::new("upper", "@").with_action(|rule, _ctx| {
//!         let body = match &rule.child_node {
//!             Value::String(text) => text.to_uppercase(),
//!             other => other.to_string().to_uppercase(),
//!         };
//!         tabnas_directive::set_node(rule, Value::String(body));
//!         Ok(())
//!     }),
//! )?;
//! # Ok::<(), tabnas_directive::DirectiveError>(())
//! ```
//!
//! TypeScript is canonical: `ts/src/directive.ts` defines behaviour,
//! option names, defaults and the order of alts. The shared fixtures in
//! `test/spec/*.tsv` are the parity contract across TypeScript, Go and
//! Rust. Intentional differences are tabulated in `docs/reference.md`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use tabnas::{
    ActionError, Context, GrammarSetting, GrammarSpec, Plugin, PluginError, Rule, RuleSnapshot,
    Tabnas, Tin, Token, Value,
};

/// VERSION is this crate's version. It MUST equal `ts/package.json`
/// "version" and the `version` field in `rs/Cargo.toml`: the release
/// orchestrator rewrites all of them, and `tests/version_test.rs` fails
/// the build if they drift. Mirrors `VERSION` in `ts/src/directive.ts`
/// and `const VERSION` in `go/directive.go`.
pub const VERSION: &str = "0.5.9";

/// The default host rules a directive modifies for its OPEN token.
const DEFAULT_OPEN_RULES: &str = "val";

/// The default host rules a directive modifies for its CLOSE token.
const DEFAULT_CLOSE_RULES: &str = "list,elem,map,pair";

/// A directive registration failure: an unusable name, a duplicate open
/// token, or a grammar that the engine refused to install.
///
/// The TypeScript plugin *throws* for the last two (it does not validate
/// the name); the Rust port returns them, exactly as the Go port does. The plugin itself never panics — a panic
/// raised inside a user callback is contained by the engine and surfaces
/// as a [`PluginError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveError(pub String);

impl fmt::Display for DirectiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DirectiveError {}

impl From<PluginError> for DirectiveError {
    fn from(error: PluginError) -> Self {
        Self(error.0)
    }
}

impl From<DirectiveError> for PluginError {
    fn from(error: DirectiveError) -> Self {
        Self(error.0)
    }
}

/// A directive action that transforms the parsed body.
///
/// The body is `rule.child_node`; the result is written to `rule.node` —
/// through [`set_node`], which handles the shared-node cell correctly.
/// Returning `Err(ActionError)` aborts the parse with that code — the Rust
/// spelling of a TypeScript action returning an *error* token.
pub type ActionFn = Arc<dyn Fn(&mut Rule, &mut Context) -> Result<(), ActionError> + Send + Sync>;

/// A directive action that may hand a token back to the engine, mirroring
/// the TypeScript `bc` hook's `Token | void` return. A token carrying an
/// error code halts the parse; any other token is forwarded and otherwise
/// ignored.
pub type TokenActionFn =
    Arc<dyn Fn(&mut Rule, &mut Context) -> Result<Option<Token>, ActionError> + Send + Sync>;

/// An alternate condition gating where a directive is recognised.
pub type ConditionFn = Arc<dyn Fn(&mut Rule, &mut Context) -> bool + Send + Sync>;

/// Extra customization of the parser once the directive rule exists.
pub type CustomFn = Arc<dyn Fn(&mut Tabnas, &DirectiveConfig) + Send + Sync>;

/// The resolved token identities for a directive, handed to a
/// [`CustomFn`] callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveConfig {
    /// The directive's OPEN token.
    pub open: Tin,
    /// The directive's CLOSE token, when it has one.
    pub close: Option<Tin>,
    /// The directive (and rule) name.
    pub name: String,
}

/// Set a rule's node value.
///
/// A rule pushed by the engine SHARES its parent's node cell, so writing
/// through `rule.node.borrow_mut()` would overwrite the parent's node too.
/// Assigning a result — what `rule.node = …` means in the canonical
/// TypeScript engine — has to install a fresh cell instead, which is what
/// this does. Reach for `rule.node.borrow_mut()` only to mutate a
/// container the rule genuinely shares (pushing onto an enclosing list,
/// say).
///
/// Every directive action wanting to produce a value should go through
/// here.
pub fn set_node(rule: &mut Rule, value: Value) {
    rule.node = Rc::new(RefCell::new(value));
}

/// How a directive transforms its parsed body.
///
/// Mirrors the TypeScript `action: StateAction | string` option: a
/// callback, or a dotted path resolved against the parser options when the
/// directive fires.
#[derive(Clone, Default)]
pub enum DirectiveAction {
    /// No action — the directive's node stays the empty map seeded by the
    /// rule's `bo` hook.
    #[default]
    None,
    /// A classic action: set `rule.node` to the result value.
    Call(ActionFn),
    /// An action that may return a token for the close hook to forward.
    Token(TokenActionFn),
    /// A dotted path resolved against the parser options at
    /// directive-execution time; the resolved value becomes the
    /// directive's node.
    ///
    /// The TypeScript options object is open, so TS resolves the path from
    /// its top level. The Rust `Options` struct is closed — like Go's —
    /// so the path is resolved in the plugin-options namespace
    /// (TS `options.plugin`): `"custom.x"` reads
    /// `parser.plugin_options("custom")["x"]`.
    Path(String),
}

impl fmt::Debug for DirectiveAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => formatter.write_str("DirectiveAction::None"),
            Self::Call(_) => formatter.write_str("DirectiveAction::Call(<function>)"),
            Self::Token(_) => formatter.write_str("DirectiveAction::Token(<function>)"),
            Self::Path(path) => write!(formatter, "DirectiveAction::Path({path:?})"),
        }
    }
}

impl DirectiveAction {
    /// Run the action in the directive rule's `bc` hook.
    fn run(&self, rule: &mut Rule, context: &mut Context) -> Result<Option<Token>, ActionError> {
        match self {
            Self::None => Ok(None),
            Self::Call(action) => action(rule, context).map(|()| None),
            Self::Token(action) => action(rule, context),
            Self::Path(path) => {
                let value = option_prop(context, path);
                set_node(rule, value);
                Ok(None)
            }
        }
    }
}

/// Resolve a dotted path within the parse's plugin-options namespace.
///
/// The lookup happens when the directive fires, against the parse's own
/// resolved options, so options set after plugin registration are seen —
/// matching the TypeScript closure over `tabnas.options`. A missing
/// segment resolves to `Value::Undefined`.
fn option_prop(context: &Context, path: &str) -> Value {
    let mut parts = path.split('.');
    let Some(head) = parts.next() else {
        return Value::Undefined;
    };
    let mut current = match context.options.plugin.get(&head.to_lowercase()) {
        Some(value) => value.clone(),
        None => return Value::Undefined,
    };
    for part in parts {
        let Value::Object(map) = current else {
            return Value::Undefined;
        };
        current = match map.get(part) {
            Some(value) => value.clone(),
            None => return Value::Undefined,
        };
    }
    current
}

/// How a directive integrates with one existing grammar rule.
#[derive(Clone, Default)]
pub struct RuleMod {
    /// An optional condition that must hold for the directive to match
    /// within this rule.
    pub c: Option<ConditionFn>,
}

impl RuleMod {
    /// A rule modification with no extra condition.
    pub fn new() -> Self {
        Self::default()
    }

    /// A rule modification gated by `condition`.
    pub fn when(
        condition: impl Fn(&mut Rule, &mut Context) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            c: Some(Arc::new(condition)),
        }
    }
}

impl fmt::Debug for RuleMod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuleMod")
            .field("c", &self.c.as_ref().map(|_| "<function>"))
            .finish()
    }
}

/// Which grammar rules a directive modifies.
///
/// `open` rules detect the directive's open token and push to the
/// directive rule; `close` rules detect the close token (when there is
/// one) so it also ends sibling parsing.
///
/// Rules are held in a [`BTreeMap`] so a directive installs its grammar in
/// a deterministic order.
#[derive(Clone, Debug, Default)]
pub struct RulesOption {
    pub open: BTreeMap<String, RuleMod>,
    pub close: BTreeMap<String, RuleMod>,
}

impl RulesOption {
    /// A rules option that modifies nothing. Passed to
    /// [`DirectiveOptions::with_rules`] it suppresses the defaults
    /// entirely — which leaves the open token unrecognised.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the open-side rules from a comma-separated list of rule names.
    pub fn open_rules(mut self, rules: &str) -> Self {
        self.open = parse_rule_list(rules);
        self
    }

    /// Set the close-side rules from a comma-separated list of rule names.
    pub fn close_rules(mut self, rules: &str) -> Self {
        self.close = parse_rule_list(rules);
        self
    }

    /// Add one open-side rule, optionally gated by a condition.
    pub fn open_rule(mut self, name: impl Into<String>, modification: RuleMod) -> Self {
        self.open.insert(name.into(), modification);
        self
    }

    /// Add one close-side rule, optionally gated by a condition.
    pub fn close_rule(mut self, name: impl Into<String>, modification: RuleMod) -> Self {
        self.close.insert(name.into(), modification);
        self
    }
}

/// Split a comma-separated rule list, dropping empty names — the Rust
/// spelling of the TypeScript `resolveRules` string form.
fn parse_rule_list(rules: &str) -> BTreeMap<String, RuleMod> {
    rules
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| (name.to_string(), RuleMod::new()))
        .collect()
}

/// Configuration for one directive.
///
/// Built with [`DirectiveOptions::new`] plus the `with_*` methods, then
/// handed to [`apply`] or [`plugin`].
#[derive(Clone)]
pub struct DirectiveOptions {
    /// The directive name, used as the rule name and token-name suffix.
    /// It must be non-empty and contain no whitespace — registration
    /// rejects anything else, because the serialized grammar could not
    /// name the token.
    pub name: String,

    /// The character sequence that starts the directive. It must be
    /// unique — re-registering an existing fixed token is an error.
    pub open: String,

    /// The optional character sequence that ends the directive. Without
    /// one the directive consumes a single value after the open token.
    pub close: Option<String>,

    /// How the parsed body is transformed.
    pub action: DirectiveAction,

    /// Which existing grammar rules are modified. `None` selects the
    /// defaults (open `val`, close `list,elem,map,pair`); `Some(value)`
    /// is a complete override, so `Some(RulesOption::new())` modifies no
    /// rules.
    ///
    /// TypeScript *deep-merges* its defaults into whatever `rules` is
    /// passed, so a partial `rules` there keeps the default of the
    /// direction it omits. Rust, like Go, cannot express that merge over a
    /// typed option and treats any `Some` as complete. This is an
    /// intentional divergence — see `docs/reference.md`.
    pub rules: Option<RulesOption>,

    /// Extra parser customization once the directive rule exists.
    pub custom: Option<CustomFn>,
}

impl fmt::Debug for DirectiveOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DirectiveOptions")
            .field("name", &self.name)
            .field("open", &self.open)
            .field("close", &self.close)
            .field("action", &self.action)
            .field("rules", &self.rules)
            .field("custom", &self.custom.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl DirectiveOptions {
    /// A directive named `name`, opened by the token source `open`.
    pub fn new(name: impl Into<String>, open: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            open: open.into(),
            close: None,
            action: DirectiveAction::None,
            rules: None,
            custom: None,
        }
    }

    /// Give the directive a close token.
    pub fn with_close(mut self, close: impl Into<String>) -> Self {
        self.close = Some(close.into());
        self
    }

    /// Set a classic action: write the result to `rule.node`.
    pub fn with_action(
        mut self,
        action: impl Fn(&mut Rule, &mut Context) -> Result<(), ActionError> + Send + Sync + 'static,
    ) -> Self {
        self.action = DirectiveAction::Call(Arc::new(action));
        self
    }

    /// Set an action that may return a token for the close hook to
    /// forward.
    pub fn with_token_action(
        mut self,
        action: impl Fn(&mut Rule, &mut Context) -> Result<Option<Token>, ActionError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        self.action = DirectiveAction::Token(Arc::new(action));
        self
    }

    /// Set a dotted-path action resolved against the plugin options when
    /// the directive fires. See [`DirectiveAction::Path`].
    pub fn with_action_path(mut self, path: impl Into<String>) -> Self {
        self.action = DirectiveAction::Path(path.into());
        self
    }

    /// Override which host rules are modified.
    pub fn with_rules(mut self, rules: RulesOption) -> Self {
        self.rules = Some(rules);
        self
    }

    /// Run `custom` once the directive rule exists.
    pub fn with_custom(
        mut self,
        custom: impl Fn(&mut Tabnas, &DirectiveConfig) + Send + Sync + 'static,
    ) -> Self {
        self.custom = Some(Arc::new(custom));
        self
    }

    /// The close token source, when there is a non-empty one.
    fn close_source(&self) -> Option<&str> {
        self.close.as_deref().filter(|close| !close.is_empty())
    }

    /// The open and close rule maps, applying the defaults when `rules` is
    /// absent.
    fn resolved_rules(&self) -> (BTreeMap<String, RuleMod>, BTreeMap<String, RuleMod>) {
        match &self.rules {
            Some(rules) => (rules.open.clone(), rules.close.clone()),
            None => (
                parse_rule_list(DEFAULT_OPEN_RULES),
                parse_rule_list(DEFAULT_CLOSE_RULES),
            ),
        }
    }
}

/// Build the directive plugin for `options`.
///
/// The returned [`Plugin`] can be installed with
/// [`Tabnas::use_plugin`], and — like every native plugin — re-runs
/// against a derived instance's options. Most callers want [`apply`].
///
/// Typed option data lives in the returned closure rather than in the
/// engine's serialized plugin-option bag, because a directive's action,
/// conditions and custom hook are Rust callbacks and no [`Value`] can
/// carry them.
pub fn plugin(options: DirectiveOptions) -> Plugin {
    Plugin::new("Directive", move |parser, _plugin_options| {
        install(parser, &options).map_err(PluginError::from)
    })
}

/// Register a directive on `parser`.
///
/// The convenience constructor mirroring the TypeScript
/// `j.use(Directive, options)` call: it forwards to
/// [`Tabnas::use_plugin`] so the directive is re-applied to derived
/// instances. Registration failures — a duplicate open token, a grammar
/// the engine refused — are returned rather than panicking.
pub fn apply(parser: &mut Tabnas, options: DirectiveOptions) -> Result<(), DirectiveError> {
    parser
        .use_plugin(plugin(options), None)
        .map(|_| ())
        .map_err(DirectiveError::from)
}

/// Install one directive's tokens, refs and grammar on `parser`.
fn install(parser: &mut Tabnas, options: &DirectiveOptions) -> Result<(), DirectiveError> {
    let name = options.name.clone();
    let open = options.open.clone();
    let close = options.close_source().map(str::to_string);
    let (open_rules, close_rules) = options.resolved_rules();

    // The name doubles as the rule name and the token-name suffix, and
    // the serialized grammar below names that token in a whitespace-split
    // `s` string, so a name holding whitespace would split into unrelated
    // tokens and the open alt could never match; an empty name has the
    // same fate. (TypeScript names tokens by resolved Tin and does not
    // validate the name.) Reject it before anything is registered, rather
    // than install a directive that silently never fires.
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(DirectiveError(format!(
            "Directive name must be non-empty and contain no whitespace: {name:?}"
        )));
    }

    // The OPEN token must be unique. (TypeScript throws here; the Rust
    // port returns the error, which `use_plugin` propagates.)
    if parser.fixed(&open).is_some() {
        return Err(DirectiveError(format!(
            "Directive open token already in use: {open}"
        )));
    }

    // Register the open fixed token.
    let open_tn = format!("#OD_{name}");
    let open_tin = parser.token_with_source(&open_tn, &open);

    // Register or look up the close fixed token. A close token already
    // claimed by another directive is reused, so `foo<…>` and `bar<…>`
    // can share `>`.
    let mut close_tin = None;
    let mut close_tn = None;
    if let Some(close) = close.as_deref() {
        if let Some(existing) = parser.fixed(close) {
            close_tin = Some(existing);
            close_tn = Some(parser.token_name(existing));
        } else {
            let token_name = format!("#CD_{name}");
            close_tin = Some(parser.token_with_source(&token_name, close));
            close_tn = Some(token_name);
        }
    }
    register_lifecycle_refs(parser, &name, options.action.clone());

    let document = build_grammar(
        parser,
        &name,
        &open_tn,
        close_tn.as_deref(),
        &open_rules,
        &close_rules,
    );

    // Clear any pre-existing alts and lifecycle actions on the directive
    // rule, so the install below lays down a clean set.
    parser.define_rule(name.clone(), |spec| {
        spec.clear();
    });

    let spec =
        GrammarSpec::from_value(document).map_err(|error| DirectiveError(error.to_string()))?;
    parser
        .grammar_with_setting(&spec, &GrammarSetting::groups("directive"))
        .map_err(|error| DirectiveError(error.to_string()))?;

    if let Some(custom) = options.custom.clone() {
        let config = DirectiveConfig {
            open: open_tin,
            // close_tin is Some exactly when close_tn is.
            close: close_tin,
            name: name.clone(),
        };
        custom(parser, &config);
    }

    Ok(())
}

/// Register the directive rule's `bo` and `bc` hooks under the engine's
/// `@<rule>-<phase>` reference convention, so installing the `<name>` rule
/// wires them automatically.
fn register_lifecycle_refs(parser: &mut Tabnas, name: &str, action: DirectiveAction) {
    // bo: seed the directive rule's node with an empty map.
    parser.state_action_with_next_ref(format!("@{name}-bo"), |rule, _context, _next, _out| {
        set_node(rule, Value::object(IndexMap::new()));
        Ok(None)
    });

    // bc: run the action. A returned token is forwarded to the engine,
    // which halts the parse when it carries an error code and otherwise
    // passes it through — exactly as the TypeScript `bc` hook does.
    parser.state_action_with_next_ref(
        format!("@{name}-bc"),
        move |rule: &mut Rule,
              context: &mut Context,
              _next: Option<&RuleSnapshot>,
              _out: Option<Token>| { action.run(rule, context) },
    );
}

/// Build the serialized grammar document covering every host-rule
/// modification plus the directive rule's own alts.
///
/// Conditions are registered on `parser` as `@dr-*` references and named
/// from the document, the Rust spelling of the TypeScript spec's inline
/// `c` functions.
fn build_grammar(
    parser: &mut Tabnas,
    name: &str,
    open_tn: &str,
    close_tn: Option<&str>,
    open_rules: &BTreeMap<String, RuleMod>,
    close_rules: &BTreeMap<String, RuleMod>,
) -> JsonValue {
    let counter = format!("dr_{name}");
    let mut rules = JsonMap::new();

    // ---- Host rules that detect the OPEN token ----

    for (rule_name, modification) in open_rules {
        let mut open_alts: Vec<JsonValue> = Vec::new();
        let mut close_alts: Vec<JsonValue> = Vec::new();

        if let Some(close_tn) = close_tn {
            // The more specific OPEN+CLOSE alt (an empty directive) is
            // tried before OPEN alone.
            open_alts.push(json!({
                "s": format!("{open_tn} {close_tn}"),
                "b": 1,
                "p": name,
                "n": { counter.clone(): 1 },
                "g": "start,end",
            }));
            close_alts.push(json!({ "s": close_tn, "b": 1, "g": "end" }));
        }

        let mut open_alt = json!({
            "s": open_tn,
            "p": name,
            "n": { counter.clone(): 1 },
            "g": "start",
        });
        if let Some(condition) = modification.c.clone() {
            let reference = format!("@dr-open-c-{name}-{rule_name}");
            parser.alt_condition(reference.clone(), move |rule, context| {
                condition(rule, context)
            });
            insert_field(&mut open_alt, "c", JsonValue::String(reference));
        }
        open_alts.push(open_alt);

        let entry = rule_entry(&mut rules, rule_name);
        insert_field(entry, "open", JsonValue::Array(open_alts));
        if !close_alts.is_empty() {
            insert_field(entry, "close", JsonValue::Array(close_alts));
        }
    }

    // ---- Host rules that detect the CLOSE token ----

    if let Some(close_tn) = close_tn {
        for (rule_name, modification) in close_rules {
            let close_reference = format!("@dr-close-c-{name}-{rule_name}");
            let condition = modification.c.clone();
            let close_counter = counter.clone();
            parser.alt_condition(close_reference.clone(), move |rule, context| {
                if rule.n.get(&close_counter).copied().unwrap_or(0) != 1 {
                    return false;
                }
                match &condition {
                    Some(condition) => condition(rule, context),
                    None => true,
                }
            });

            let comma_reference = format!("@dr-close-ca-c-{name}-{rule_name}");
            let comma_counter = counter.clone();
            parser.alt_condition(comma_reference.clone(), move |rule, _context| {
                rule.n.get(&comma_counter).copied().unwrap_or(0) == 1
            });

            let entry = rule_entry(&mut rules, rule_name);
            insert_field(
                entry,
                "close",
                json!([
                    { "s": close_tn, "c": close_reference, "b": 1, "g": "end" },
                    {
                        "s": format!("#CA {close_tn}"),
                        "c": comma_reference,
                        "b": 1,
                        "g": "end,comma",
                    },
                ]),
            );
        }
    }

    // ---- The directive rule's own alts ----

    let mut directive_open: Vec<JsonValue> = Vec::new();
    if let Some(close_tn) = close_tn {
        // An immediate close is an empty directive.
        directive_open.push(json!({ "s": close_tn, "b": 1 }));
    }
    // Push to `val` to parse the directive's body. The counters control
    // implicit list/map creation: with a close token implicits are
    // permitted (they are bounded by the close), without one they are
    // suppressed so the directive does not eat its following siblings.
    let counters = if close_tn.is_some() {
        json!({ "dlist": 0, "dmap": 0 })
    } else {
        json!({ "dlist": 1, "dmap": 1 })
    };
    directive_open.push(json!({ "p": "val", "n": counters }));

    let entry = rule_entry(&mut rules, name);
    insert_field(entry, "open", JsonValue::Array(directive_open));
    if let Some(close_tn) = close_tn {
        insert_field(
            entry,
            "close",
            json!([{ "s": close_tn }, { "s": format!("#CA {close_tn}") }]),
        );
    }

    json!({ "rule": JsonValue::Object(rules) })
}

/// The grammar document entry for one rule, created on first use. A rule
/// named in both the open and close sets reuses the same entry.
fn rule_entry<'a>(rules: &'a mut JsonMap<String, JsonValue>, name: &str) -> &'a mut JsonValue {
    rules
        .entry(name.to_string())
        .or_insert_with(|| JsonValue::Object(JsonMap::new()))
}

/// Set one field on a JSON object built above. Every call site constructs
/// the object itself, so the target is always an object.
fn insert_field(target: &mut JsonValue, key: &str, value: JsonValue) {
    if let Some(object) = target.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

import { Jsonic, StateAction, Plugin, Tin } from 'jsonic';
type DirectiveOptions = {
    name: string;
    open: string;
    action: StateAction | string;
    close?: string;
    rules?: {
        open?: string | string[] | Record<string, {
            c?: Function;
        }>;
        close?: string | string[] | Record<string, {
            c?: Function;
        }>;
    };
    custom?: (jsonic: Jsonic, config: {
        OPEN: Tin;
        CLOSE: Tin | null | undefined;
        name: string;
    }) => void;
};
declare const Directive: Plugin;
export { Directive };
export type { DirectiveOptions };

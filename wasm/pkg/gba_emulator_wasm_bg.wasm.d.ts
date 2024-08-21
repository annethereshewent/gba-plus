/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export function __wbg_wasmemulator_free(a: number, b: number): void;
export function wasmemulator_new(): number;
export function wasmemulator_load_save(a: number, b: number, c: number): void;
export function wasmemulator_backup_file_pointer(a: number): number;
export function wasmemulator_backup_file_size(a: number): number;
export function wasmemulator_has_saved(a: number): number;
export function wasmemulator_set_saved(a: number, b: number): void;
export function wasmemulator_update_buffer(a: number, b: number, c: number, d: number, e: number, f: number, g: number): void;
export function wasmemulator_step_frame(a: number): void;
export function wasmemulator_get_picture_pointer(a: number): number;
export function wasmemulator_load(a: number, b: number, c: number): void;
export function wasmemulator_load_bios(a: number, b: number, c: number): void;
export function wasmemulator_update_input(a: number, b: number, c: number): void;
export function __wbindgen_malloc(a: number, b: number): number;

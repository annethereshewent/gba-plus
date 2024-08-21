/* tslint:disable */
/* eslint-disable */
/**
*/
export enum ButtonEvent {
  ButtonA = 0,
  ButtonB = 1,
  ButtonL = 2,
  ButtonR = 3,
  Select = 4,
  Start = 5,
  Up = 6,
  Down = 7,
  Left = 8,
  Right = 9,
}
/**
*/
export class WasmEmulator {
  free(): void;
/**
*/
  constructor();
/**
* @param {Uint8Array} data
*/
  load_save(data: Uint8Array): void;
/**
* @returns {number}
*/
  backup_file_pointer(): number;
/**
* @returns {number}
*/
  backup_file_size(): number;
/**
* @returns {boolean}
*/
  has_saved(): boolean;
/**
* @param {boolean} val
*/
  set_saved(val: boolean): void;
/**
* @param {Float32Array} left_buffer
* @param {Float32Array} right_buffer
*/
  update_buffer(left_buffer: Float32Array, right_buffer: Float32Array): void;
/**
*/
  step_frame(): void;
/**
* @returns {number}
*/
  get_picture_pointer(): number;
/**
* @param {Uint8Array} rom
*/
  load(rom: Uint8Array): void;
/**
* @param {Uint8Array} bios
*/
  load_bios(bios: Uint8Array): void;
/**
* @param {ButtonEvent} button_event
* @param {boolean} is_pressed
*/
  update_input(button_event: ButtonEvent, is_pressed: boolean): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_wasmemulator_free: (a: number, b: number) => void;
  readonly wasmemulator_new: () => number;
  readonly wasmemulator_load_save: (a: number, b: number, c: number) => void;
  readonly wasmemulator_backup_file_pointer: (a: number) => number;
  readonly wasmemulator_backup_file_size: (a: number) => number;
  readonly wasmemulator_has_saved: (a: number) => number;
  readonly wasmemulator_set_saved: (a: number, b: number) => void;
  readonly wasmemulator_update_buffer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
  readonly wasmemulator_step_frame: (a: number) => void;
  readonly wasmemulator_get_picture_pointer: (a: number) => number;
  readonly wasmemulator_load: (a: number, b: number, c: number) => void;
  readonly wasmemulator_load_bios: (a: number, b: number, c: number) => void;
  readonly wasmemulator_update_input: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

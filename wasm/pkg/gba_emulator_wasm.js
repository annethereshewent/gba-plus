let wasm;

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

let cachedFloat32ArrayMemory0 = null;

function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}
/**
*/
export const ButtonEvent = Object.freeze({ ButtonA:0,"0":"ButtonA",ButtonB:1,"1":"ButtonB",ButtonL:2,"2":"ButtonL",ButtonR:3,"3":"ButtonR",Select:4,"4":"Select",Start:5,"5":"Start",Up:6,"6":"Up",Down:7,"7":"Down",Left:8,"8":"Left",Right:9,"9":"Right", });

const WasmEmulatorFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmemulator_free(ptr >>> 0, 1));
/**
*/
export class WasmEmulator {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmEmulatorFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmemulator_free(ptr, 0);
    }
    /**
    */
    constructor() {
        const ret = wasm.wasmemulator_new();
        this.__wbg_ptr = ret >>> 0;
        WasmEmulatorFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
    * @param {Uint8Array} data
    */
    load_save(data) {
        const ptr0 = passArray8ToWasm0(data, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmemulator_load_save(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @returns {number}
    */
    backup_file_pointer() {
        const ret = wasm.wasmemulator_backup_file_pointer(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
    * @returns {number}
    */
    backup_file_size() {
        const ret = wasm.wasmemulator_backup_file_size(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
    * @returns {boolean}
    */
    has_saved() {
        const ret = wasm.wasmemulator_has_saved(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
    * @param {boolean} val
    */
    set_saved(val) {
        wasm.wasmemulator_set_saved(this.__wbg_ptr, val);
    }
    /**
    * @param {Float32Array} left_buffer
    * @param {Float32Array} right_buffer
    */
    update_buffer(left_buffer, right_buffer) {
        var ptr0 = passArrayF32ToWasm0(left_buffer, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        var ptr1 = passArrayF32ToWasm0(right_buffer, wasm.__wbindgen_malloc);
        var len1 = WASM_VECTOR_LEN;
        wasm.wasmemulator_update_buffer(this.__wbg_ptr, ptr0, len0, addHeapObject(left_buffer), ptr1, len1, addHeapObject(right_buffer));
    }
    /**
    */
    step_frame() {
        wasm.wasmemulator_step_frame(this.__wbg_ptr);
    }
    /**
    * @returns {number}
    */
    get_picture_pointer() {
        const ret = wasm.wasmemulator_get_picture_pointer(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
    * @param {Uint8Array} rom
    */
    load(rom) {
        const ptr0 = passArray8ToWasm0(rom, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmemulator_load(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @param {Uint8Array} bios
    */
    load_bios(bios) {
        const ptr0 = passArray8ToWasm0(bios, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.wasmemulator_load_bios(this.__wbg_ptr, ptr0, len0);
    }
    /**
    * @param {ButtonEvent} button_event
    * @param {boolean} is_pressed
    */
    update_input(button_event, is_pressed) {
        wasm.wasmemulator_update_input(this.__wbg_ptr, button_event, is_pressed);
    }
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_copy_to_typed_array = function(arg0, arg1, arg2) {
        new Uint8Array(getObject(arg2).buffer, getObject(arg2).byteOffset, getObject(arg2).byteLength).set(getArrayU8FromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedFloat32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;



    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined' && Object.getPrototypeOf(module) === Object.prototype)
    ({module} = module)
    else
    console.warn('using deprecated parameters for `initSync()`; pass a single object instead')

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined' && Object.getPrototypeOf(module_or_path) === Object.prototype)
    ({module_or_path} = module_or_path)
    else
    console.warn('using deprecated parameters for the initialization function; pass a single object instead')

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('gba_emulator_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;

import { InitOutput, WasmEmulator } from "../../pkg/gba_emulator_wasm"

export const SCREEN_WIDTH = 240
export const SCREEN_HEIGHT = 160

export class Renderer {
  emulator: WasmEmulator
  wasm: InitOutput
  context = (document.getElementById("canvas") as HTMLCanvasElement).getContext("2d")

  constructor(emulator: WasmEmulator, wasm: InitOutput) {
    this.emulator = emulator
    this.wasm = wasm
  }

  getImageData() {
    const rustMemory = new Uint8Array(this.wasm.memory.buffer,this.emulator.get_picture_pointer())

    const imageData = this.context!.getImageData(0,0, SCREEN_WIDTH, SCREEN_HEIGHT);

    for (let x = 0; x < SCREEN_WIDTH; x++) {
      for (let y = 0; y < SCREEN_HEIGHT; y++) {
        const imageIndex = x * 4 + y * SCREEN_WIDTH * 4;

        imageData.data[imageIndex] = rustMemory[imageIndex]
        imageData.data[imageIndex+1] = rustMemory[imageIndex+1]
        imageData.data[imageIndex+2] = rustMemory[imageIndex+2]
        imageData.data[imageIndex+3] = rustMemory[imageIndex+3]
      }
    }

    return imageData
  }

  render() {
    this.context?.putImageData(this.getImageData(), 0, 0)
  }

}
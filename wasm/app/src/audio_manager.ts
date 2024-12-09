import { WasmEmulator } from "../../pkg/gba_emulator_wasm"


const SAMPLE_RATE = 44100
const BUFFER_SIZE = 4096

export class AudioManager {
  emulator: WasmEmulator

  constructor(emulator: WasmEmulator) {
    this.emulator = emulator
  }
  startAudio() {
    const audioContext = new AudioContext({ sampleRate: SAMPLE_RATE })

    const scriptProcessor = audioContext.createScriptProcessor(BUFFER_SIZE, 0, 2);

    scriptProcessor.onaudioprocess = (e) => {
      const leftData = e.outputBuffer.getChannelData(0)
      const rightData = e.outputBuffer.getChannelData(1)

      this.emulator.update_buffer(leftData, rightData)
    }

    scriptProcessor.connect(audioContext.destination)
  }
}
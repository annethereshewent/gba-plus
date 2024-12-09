import init, { WasmEmulator, InitOutput } from "../../pkg/gba_emulator_wasm.js"
import wasmData from '../../pkg/gba_emulator_wasm_bg.wasm'
import JSZip from "jszip"
import { AudioManager } from "./audio_manager"
import { Renderer } from "./renderer"
import { Joypad } from "./joypad"

const FPS_INTERVAL = 1000 / 60

export class UI {
  emulator: WasmEmulator
  audioManager: AudioManager
  renderer: Renderer|null = null
  fileName = ""
  frames = 0
  previousTime = 0
  wasm: InitOutput|null = null
  joypad: Joypad

  constructor() {
    this.emulator = new WasmEmulator()
    this.audioManager = new AudioManager(this.emulator)
    this.joypad = new Joypad(this.emulator)

    const romInput = document.getElementById("game-input")
    const biosInput = document.getElementById("bios-input")

    romInput!.addEventListener("change", (e) => {
      this.handleFileChange(e)
    })

    biosInput!.addEventListener("change", (e) => {
      this.handleBiosChange(e)
    })
  }

  addEventListeners() {
    console.log("adding event listeners")
    document.getElementById("game-button")!.addEventListener("click", () => this.loadRom())

    document.getElementById("close-btn")!.addEventListener("click", () => this.hideHelpModal())

    document.getElementById("help-btn")!.addEventListener("click", () => this.showHelpModal())
    document.getElementById("load-bios-btn")!.addEventListener("click", () => this.loadBios())

    document.getElementById("full-screen")!.addEventListener("click", (e) => document.documentElement.requestFullscreen())
  }

  async init() {
    console.log("initializing shit")

    this.wasm = await init(wasmData)

    this.renderer = new Renderer(this.emulator, this.wasm)

    const biosJson = JSON.parse(localStorage.getItem('gba_bios') ?? "null")

    if (biosJson != null) {
      this.emulator.load_bios(new Uint8Array(biosJson))
      document.getElementById("load-game-btn")!.removeAttribute("disabled")
      document.getElementById("load-bios-btn")!.setAttribute("disabled", "true")
    } else {
      // load open source bios instead
      const biosResponse = await fetch("./bios/gba_opensource_bios.bin")
      const biosBody = await biosResponse.arrayBuffer()

      this.emulator.load_bios(new Uint8Array(biosBody))
    }
  }

  loadRom() {
    console.log("loading rom")
    document.getElementById("game-input")?.click()
  }

  loadBios() {
    document.getElementById("bios-input")?.click()
  }

  showHelpModal() {
    document.getElementById("modal")!.style.display = "block"
  }

  hideHelpModal() {
    document.getElementById("modal")!.style.display = "none"
  }

  async handleFileChange(e: Event) {
    let rom = await this.getBinaryData(e)

    if (rom != null) {
      this.emulator.load(new Uint8Array(rom))

      let gameName = this.fileName.split('/').pop()
      gameName = gameName?.substring(0, gameName.lastIndexOf('.'))

      let saveData = JSON.parse(localStorage.getItem(gameName ?? "") ?? "null")

      if (saveData != null) {
        this.emulator.load_save(new Uint8Array(saveData))
      }

      this.audioManager.startAudio()
      requestAnimationFrame((time) => this.run(time))
    }
  }

  async handleBiosChange(e: Event) {
    let bios = await this.getBinaryData(e)

    if (bios != null) {
      const biosUintArray = new Uint8Array(bios)
      this.emulator.load_bios(biosUintArray);

      const toast = document.getElementById("toast")
      toast!.style.display = "block"

      document.getElementById("load-game-btn")!.removeAttribute("disabled")
      document.getElementById("load-bios-btn")!.setAttribute("disabled", "true")

      localStorage.setItem("gba_bios", JSON.stringify(Array.from(biosUintArray)))

      setTimeout(() => toast!.style.display = "none", 1000)
    }
  }

  async getBinaryData(e: Event) {
    let data: ArrayBuffer|undefined = undefined
    if ((e.target as HTMLInputElement).files != null) {
      const file = (e.target as HTMLInputElement).files![0]
      this.fileName = file.name
      if (file.name.indexOf(".zip") !== -1) {
        // unzip the file first
        const zipFile = await JSZip.loadAsync(file)
        const zipFileName = Object.keys(zipFile.files)[0]

        data = await zipFile?.file(zipFileName)?.async('arraybuffer')
      } else {
        data = await this.fileToArrayBuffer(file)
      }
    }
    return data
  }

  fileToArrayBuffer(file: File): Promise<ArrayBuffer> {
    const fileReader = new FileReader()

    return new Promise((resolve, reject) => {
      fileReader.onload = () => resolve(fileReader.result as ArrayBuffer)

      fileReader.onerror = () => {
        fileReader.abort()
        reject(new Error("Error parsing file"))
      }

      fileReader.readAsArrayBuffer(file)
    })
  }

  run(time: number) {
    const diff = time - this.previousTime

    const fps = Math.floor(1000 / diff)

    if (diff >= FPS_INTERVAL || this.previousTime == 0) {
      this.previousTime = time - (diff % FPS_INTERVAL)

      if (this.frames % 60 == 0) {
        document.getElementById("fps-counter")!.innerText = `FPS = ${fps}`
      }

      this.emulator.step_frame()
      this.renderer!.render()

      this.joypad.handleJoypadInput()
    }
    if (this.emulator.has_saved()) {
      this.emulator.set_saved(false)

      const saveMemory = new Uint8Array(this.wasm!.memory.buffer, this.emulator.backup_file_pointer(), this.emulator.backup_file_size())

      let gameName = this.fileName.split('/').pop()
      gameName = gameName!.substring(0, gameName!.lastIndexOf('.'))

      localStorage.setItem(gameName, JSON.stringify(Array.from(saveMemory)))
    }

    this.frames++

    requestAnimationFrame((time) => this.run(time))
  }
}
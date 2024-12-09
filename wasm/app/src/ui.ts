import init, { WasmEmulator, InitOutput } from "../../pkg/gba_emulator_wasm.js"
import wasmData from '../../pkg/gba_emulator_wasm_bg.wasm'
import JSZip from "jszip"
import { AudioManager } from "./audio_manager"
import { Renderer } from "./renderer"
import { Joypad } from "./joypad"
import { CloudService } from "./cloud_service"

const FPS_INTERVAL = 1000 / 60

export class UI {
  emulator: WasmEmulator|null = null
  audioManager: AudioManager|null = null
  renderer: Renderer|null = null
  fileName = ""
  frames = 0
  previousTime = 0
  wasm: InitOutput|null = null
  joypad: Joypad|null = null
  cloudService = new CloudService()
  updateSaveGame = ""

  constructor() {
    this.init()

    const romInput = document.getElementById("game-input")
    const biosInput = document.getElementById("bios-input")

    romInput!.addEventListener("change", (e) => {
      this.handleFileChange(e)
    })

    biosInput!.addEventListener("change", (e) => {
      this.handleBiosChange(e)
    })
  }

  checkOauth() {
    this.cloudService.checkAuthentication()
  }

  addEventListeners() {
    document.getElementById("game-button")?.addEventListener("click", () => this.loadRom())
    document.getElementById("load-bios-btn")?.addEventListener("click", () => this.loadBios())
    document.getElementById("save-management")?.addEventListener("click", () => this.displaySavesModal())
    document.getElementById("save-input")?.addEventListener("change", (e) => this.handleSaveChange(e))
  }

  async init() {
    this.wasm = await init(wasmData)

    this.emulator = new WasmEmulator()
    this.audioManager = new AudioManager(this.emulator)
    this.joypad = new Joypad(this.emulator)

    this.renderer = new Renderer(this.emulator, this.wasm)

    const biosJson = JSON.parse(localStorage.getItem('gba_bios') ?? "null")

    if (biosJson != null) {
      this.emulator.load_bios(new Uint8Array(biosJson))
      document.getElementById("game-button")!.removeAttribute("disabled")
      document.getElementById("load-bios-btn")!.setAttribute("disabled", "true")
    } else {
      // load open source bios instead
      const biosResponse = await fetch("./bios/gba_opensource_bios.bin")
      const biosBody = await biosResponse.arrayBuffer()

      this.emulator.load_bios(new Uint8Array(biosBody))
    }
  }

  loadRom() {
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

  async handleSaveChange(e: Event) {
    if (!this.cloudService.usingCloud) {
      return
    }
    let saveName = (e.target as HTMLInputElement)?.files?.[0].name?.split('/')?.pop()

    if (saveName != this.updateSaveGame) {
      if (!confirm("Warning! Save file does not match selected game name. are you sure you want to continue?")) {
        return
      }
    }
    const data = await this.getBinaryData(e)

    if (data != null) {
      const bytes = new Uint8Array(data as ArrayBuffer)

      if (this.updateSaveGame != "") {
        this.cloudService.uploadSave(this.updateSaveGame, bytes)
      }

      const notification = document.getElementById("save-notification")

      if (notification != null) {
        notification.style.display = "block"

        let opacity = 1.0

        let interval = setInterval(() => {
          opacity -= 0.1
          notification.style.opacity = `${opacity}`

          if (opacity <= 0) {
            clearInterval(interval)
          }
        }, 100)
      }

      const savesModal = document.getElementById("saves-modal")

      if (savesModal != null) {
        savesModal.style.display = "none"
        savesModal.className = "modal hide"
      }
    }
  }

  async displaySavesModal() {
    if (!this.cloudService.usingCloud) {
      return
    }
    const saves = await this.cloudService.getSaves()
    const savesModal = document.getElementById("saves-modal")
    const savesList = document.getElementById("saves-list")

    if (saves != null && savesModal != null && savesList != null) {
      savesModal.className = "modal show"
      savesModal.style.display = "block"

      // this.emulator?.set_pause(true)

      savesList.innerHTML = ''
      for (const save of saves) {
        const divEl = document.createElement("div")

        divEl.className = "save-entry"

        const spanEl = document.createElement("span")

        spanEl.innerText = save.gameName.length > 50 ? save.gameName.substring(0, 50) + "..." : save.gameName

        const deleteSaveEl = document.createElement('i')

        deleteSaveEl.className = "fa-solid fa-x save-icon delete-save"

        deleteSaveEl.addEventListener('click', () => this.deleteSave(save.gameName))

        const updateSaveEl = document.createElement('i')

        updateSaveEl.className = "fa-solid fa-file-pen save-icon update"

        updateSaveEl.addEventListener("click", () => this.updateSave(save.gameName))

        const downloadSaveEl = document.createElement("div")

        downloadSaveEl.className = "fa-solid fa-download save-icon download"

        downloadSaveEl.addEventListener("click", () => this.downloadSave(save.gameName))

        divEl.append(spanEl)
        divEl.append(downloadSaveEl)
        divEl.append(deleteSaveEl)
        divEl.append(updateSaveEl)

        savesList.append(divEl)
      }
    }
  }

  async downloadSave(gameName: string) {
    if (!this.cloudService.usingCloud) {
      return
    }
    const entry = await this.cloudService.getSave(gameName)

    if (entry != null) {
      this.generateFile(entry.data!!, gameName)
    }
  }

  updateSave(gameName: string) {
    this.updateSaveGame = gameName

    document.getElementById("save-input")?.click()
  }

  async deleteSave(gameName: string) {
    if (this.cloudService.usingCloud && confirm("are you sure you want to delete this save?")) {
      const result = await this.cloudService.deleteSave(gameName)

      if (result) {
        const savesList = document.getElementById("saves-list")

        if (savesList != null) {
          for (const child of savesList.children) {
            const children = [...child.children]
            const spanElement = (children.filter((childEl) => childEl.tagName.toLowerCase() == 'span')[0] as HTMLSpanElement)

            if (spanElement?.innerText == gameName) {
              child.remove()
              break
            }
          }
        }
      }
    }
  }

  generateFile(data: Uint8Array, gameName: string) {
    const blob = new Blob([data], {
      type: "application/octet-stream"
    })

    const objectUrl = URL.createObjectURL(blob)

    const a = document.createElement('a')

    a.href = objectUrl
    a.download = gameName.match(/\.sav$/) ? gameName : `${gameName}.sav`
    document.body.append(a)
    a.style.display = "none"

    a.click()
    a.remove()

    setTimeout(() => URL.revokeObjectURL(objectUrl), 1000)
  }

  async handleFileChange(e: Event) {
    let rom = await this.getBinaryData(e)

    if (rom != null) {
      this.emulator!.load(new Uint8Array(rom))

      let gameName = this.fileName.split('/').pop()
      gameName = gameName?.substring(0, gameName.lastIndexOf('.'))

      let saveData = this.cloudService.usingCloud ? (await this.cloudService.getSave(gameName!)).data : new Uint8Array(JSON.parse(localStorage.getItem(gameName ?? "") ?? "null"))

      if (saveData != null) {
        this.emulator!.load_save(saveData)
      }

      this.audioManager!.startAudio()
      requestAnimationFrame((time) => this.run(time))
    }
  }

  async handleBiosChange(e: Event) {
    let bios = await this.getBinaryData(e)

    if (bios != null) {
      const biosUintArray = new Uint8Array(bios)
      this.emulator!.load_bios(biosUintArray);

      const toast = document.getElementById("bios-notification")
      toast!.style.display = "block"

      document.getElementById("load-bios-btn")?.setAttribute("disabled", "true")

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

    if (this.frames == 60) {
      this.frames = 0
      document.getElementById("fps-counter")!.innerText = `FPS = ${fps}`
    }

    if (diff >= FPS_INTERVAL || this.previousTime == 0) {
      this.previousTime = time - (diff % FPS_INTERVAL)

      this.emulator!.step_frame()
      this.renderer!.render()

      this.joypad!.handleJoypadInput()
    }
    if (this.emulator!.has_saved()) {
      this.emulator!.set_saved(false)

      const saveMemory = new Uint8Array(this.wasm!.memory.buffer, this.emulator!.backup_file_pointer(), this.emulator!.backup_file_size())

      let gameName = this.fileName.split('/').pop()
      gameName = gameName!.substring(0, gameName!.lastIndexOf('.'))

      const clonedSave = new Uint8Array(Array.from(saveMemory))

      this.cloudService.usingCloud ?  this.cloudService.uploadSave(gameName, clonedSave) : localStorage.setItem(gameName, JSON.stringify(Array.from(saveMemory)))
    }

    this.frames++

    requestAnimationFrame((time) => this.run(time))
  }
}
import init, { WasmEmulator, InitOutput } from "../../pkg/gba_emulator_wasm.js"
import wasmData from '../../pkg/gba_emulator_wasm_bg.wasm'
import JSZip from "jszip"
import { AudioManager } from "./audio_manager"
import { Renderer, SCREEN_HEIGHT, SCREEN_WIDTH } from "./renderer"
import { Joypad } from "./joypad"
import { CloudService } from "./cloud_service"
import { GbaDatabase } from "./gba_database"
import moment from "moment"
import { StateEntry } from "./game_state_entry"
import { StateManager } from "./state_manager"

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
  db = new GbaDatabase()
  stateManager: StateManager|null = null
  biosData: Uint8Array|null = null
  gameData: Uint8Array|null = null
  gameName = ""

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

  async createSaveState() {
    const now = moment()

    const stateName = `${now.unix()}.state`

    if (this.gameName != "") {
      const imageUrl = this.getImageUrl()
      if (imageUrl != null) {
        const entry = await this.stateManager?.createSaveState(imageUrl, stateName)
        const statesList = document.getElementById("states-list")

        if (entry != null && statesList != null) {
          this.addStateElement(statesList, entry)
        }
      }
    }
  }

  addEventListeners() {
    document.getElementById("game-button")?.addEventListener("click", () => this.loadRom())
    document.getElementById("load-bios-btn")?.addEventListener("click", () => this.loadBios())
    document.getElementById("save-management")?.addEventListener("click", () => this.displaySavesModal())
    document.getElementById("save-input")?.addEventListener("change", (e) => this.handleSaveChange(e))
    document.getElementById("save-states")?.addEventListener("click", () => this.displaySaveStatesModal())
    document.getElementById("create-save-state")?.addEventListener("click", () => this.createSaveState())
    document.getElementById("states-modal-close")?.addEventListener("click", () => this.closeStatesModal())
    document.getElementById("hide-saves-modal")?.addEventListener("click", () => this.closeSavesModal())
  }

  closeSavesModal() {
    this.emulator?.set_pause(false)
    const savesModal = document.getElementById("saves-modal")

    if (savesModal != null) {
      savesModal.className = "modal hide"
      savesModal.style.display = "none"
    }
  }

  async init() {
    this.wasm = await init(wasmData)

    this.emulator = new WasmEmulator()
    this.audioManager = new AudioManager(this.emulator)
    this.joypad = new Joypad(this.emulator, this)
    this.renderer = new Renderer(this.emulator, this.wasm)

    const biosJson = JSON.parse(localStorage.getItem('gba_bios') ?? "null")

    let biosData = new Uint8Array()

    if (biosJson != null) {
      biosData = new Uint8Array(biosJson)

      document.getElementById("load-bios-btn")!.setAttribute("disabled", "true")
    } else {
      // load open source bios instead
      const biosResponse = await fetch("./bios/gba_opensource_bios.bin")
      const biosBody = await biosResponse.arrayBuffer()

      biosData = new Uint8Array(biosBody)
    }

    this.emulator.load_bios(biosData)
    this.biosData = biosData
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

  async displaySaveStatesModal() {
    if (this.gameName != "") {
      const modal = document.getElementById("states-modal")
      const statesList = document.getElementById("states-list")

      if (modal != null && statesList != null) {
        this.emulator?.set_pause(true)
        modal.style.display = "block"

        statesList.innerHTML = ""

        const entry = await this.db.getSaveStates(this.gameName)

        if (entry != null) {
          for (const key in entry.states) {
            const stateEntry = entry.states[key]

            this.addStateElement(statesList, stateEntry)
          }
        }
      }
    }
  }

  displayMenu(stateName: string) {
    const menus = document.getElementsByClassName("state-menu") as HTMLCollectionOf<HTMLElement>

    for (const menu of menus) {
      if (menu.id.indexOf(stateName) == -1) {
        menu.style.display = "none"
      }
    }

    const menu = document.getElementById(`menu-${stateName}`)

    if (menu != null) {
      if (menu.style.display == "block") {
        menu.style.display = "none"
      } else {
        menu.style.display = "block"
      }
    }
  }

  addStateElement(statesList: HTMLElement, entry: StateEntry) {
    const divEl = document.createElement("div")

    divEl.className = "state-element"
    divEl.id = entry.stateName

    divEl.addEventListener("click", () => this.displayMenu(entry.stateName))

    const imgEl = document.createElement("img")

    imgEl.className = "state-image"
    imgEl.id = `image-${entry.stateName}`

    const pEl = document.createElement("p")
    pEl.id = `title-${entry.stateName}`

    if (entry.stateName != "quick_save.state") {

      const timestamp = parseInt(entry.stateName.replace(".state", ""))

      pEl.innerText = `Save on ${moment.unix(timestamp).format("lll")}`
    } else {
      pEl.innerText = "Quick save"
    }

    const menu = document.createElement("aside")

    menu.className = "state-menu hide"
    menu.id = `menu-${entry.stateName}`
    menu.style.display = "none"

    menu.innerHTML = `
      <ul class="state-menu-list">
        <li><a id="update-${entry.stateName}">Update State</a></li>
        <li><a id="load-${entry.stateName}">Load state</a></li>
        <li><a id="delete-${entry.stateName}">Delete state</a></li>
      </ul>
    `
    imgEl.src = entry.imageUrl


    divEl.append(imgEl)
    divEl.append(pEl)
    divEl.append(menu)

    statesList.append(divEl)

    // finally add event listeners for loading and deleting states
    document.getElementById(`update-${entry.stateName}`)?.addEventListener("click", () => this.updateState(entry))
    document.getElementById(`load-${entry.stateName}`)?.addEventListener("click", () => this.loadSaveState(entry.state))
    document.getElementById(`delete-${entry.stateName}`)?.addEventListener("click", () => this.deleteState(entry.stateName))
  }

  updateStateElement(entry: StateEntry, oldStateName: string) {
    const image = document.getElementById(`image-${oldStateName}`) as HTMLImageElement
    const title = document.getElementById(`title-${oldStateName}`)

    if (image != null && title != null) {
      image.src = entry.imageUrl

      if (entry.stateName != "quick_save.state") {
        const timestamp = parseInt(entry.stateName.replace(".state", ""))

        title.innerText = `Save on ${moment.unix(timestamp).format("lll")}`
      }
    }
  }

  getImageUrl() {
    if (this.emulator != null && this.wasm != null) {
      let screen = new Uint8Array(SCREEN_WIDTH * SCREEN_HEIGHT * 4)
      screen = new Uint8Array(this.wasm.memory.buffer, this.emulator.get_picture_pointer(), SCREEN_WIDTH * SCREEN_HEIGHT * 4)
      const canvas = document.getElementById("save-state-canvas") as HTMLCanvasElement

      const context = canvas.getContext("2d")

      if (context != null) {
        const imageData = context.getImageData(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT)

        let screenIndex = 0
        for (let i = 0; i < screen.length; i += 4) {
          imageData.data[i] = screen[screenIndex]
          imageData.data[i + 1] = screen[screenIndex + 1]
          imageData.data[i + 2] = screen[screenIndex + 2]
          imageData.data[i + 3] = screen[screenIndex + 3]

          screenIndex += 4
        }

        context.putImageData(imageData, 0, 0)

        return canvas.toDataURL()
      }
    }

    return null
  }

  async updateState(entry: StateEntry) {
    const imageUrl = this.getImageUrl()
    if (imageUrl != null && this.stateManager != null) {
      const oldStateName = entry.stateName

      const updateEntry = await this.stateManager.createSaveState(imageUrl, entry.stateName, true)

      if (updateEntry != null) {
        this.updateStateElement(updateEntry, oldStateName)
      }
    }
  }

  async loadSaveState(compressed: Uint8Array) {
    if (this.biosData != null && this.gameData != null) {
      this.emulator?.set_pause(true)
      if (this.emulator != null && this.stateManager != null) {
        const data = await this.stateManager.decompress(compressed)

        if (data != null) {
          this.emulator.load_save_state(data)


          this.emulator.load_bios(this.biosData)

          this.emulator.reload_rom(this.gameData)
        }

        this.closeStatesModal()
      }
    }
  }

  showStateCreatedNotification() {
    const notification = document.getElementById("state-notification")

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
  }

  closeStatesModal() {
    this.emulator?.set_pause(false)
    const statesModal = document.getElementById("states-modal")

    if (statesModal != null) {
      statesModal.className = "modal hide"
      statesModal.style.display = "none"
    }
  }

  async deleteState(stateName: string) {
    if (confirm("Are you sure you want to delete this save state?")) {
      await this.db.deleteState(this.fileName.substring(0, this.fileName.lastIndexOf('.')), stateName)

      const el = document.getElementById(stateName)

      el?.remove()
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

      this.emulator?.set_pause(true)

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
      this.gameData = new Uint8Array(rom)

      this.gameName = this.fileName.split('/').pop() ?? ""
      this.gameName = this.gameName.substring(0, this.gameName.lastIndexOf('.'))

      this.stateManager = new StateManager(this.emulator!, this.wasm, this.gameName, this.db)

      let saveData = this.cloudService.usingCloud ? (await this.cloudService.getSave(this.gameName!)).data : new Uint8Array(JSON.parse(localStorage.getItem(this.gameName) ?? "null"))

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
      this.emulator!.load_bios(biosUintArray)

      this.biosData = biosUintArray

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
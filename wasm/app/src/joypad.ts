import { WasmEmulator, ButtonEvent } from "../../pkg/gba_emulator_wasm"


const A_BUTTON = 0
const B_BUTTON = 1
const X_BUTTON = 2
const L_BUTTON = 4
const R_BUTTON = 5
const SELECT = 8
const START = 9
const UP = 12
const DOWN = 13
const LEFT = 14
const RIGHT = 15

export class Joypad {
  emulator: WasmEmulator
  isPressingW = false
  isPressingA = false
  isPressingS = false
  isPressingD = false

  isPressingC = false
  isPressingV = false

  isPressingJ = false
  isPressingK = false

  isPressingSpace = false
  isPressingEnter = false
  isPressingTab = false
  isPressingShift = false

  constructor(emulator: WasmEmulator) {
    this.emulator = emulator
    this.addKeyboardListeners()
  }

  addKeyboardListeners() {
    document.addEventListener("keyup", (e) => {
      switch (e.key) {
        case "w":
          this.isPressingW = false
          break
        case "a":
          this.isPressingA = false
          break
        case "s":
          this.isPressingS = false
          break
        case "d":
          this.isPressingD = false
          break
        case "j":
          this.isPressingJ = false
          break
        case "k":
          this.isPressingK = false
          break
        case "Enter":
          e.preventDefault()
          this.isPressingEnter = false
          break
        case "Tab":
          e.preventDefault()
          this.isPressingTab = false
          break
        case "Space":
          e.preventDefault()
          this.isPressingSpace = false
          break
        case "Shift":
          e.preventDefault()
          this.isPressingShift = false
          break
        case "c":
          this.isPressingC = false
          break
        case "v":
          this.isPressingV = false
          break
      }
    })

    document.addEventListener("keydown", (e) => {
      switch (e.key) {
        case "w":
          this.isPressingW = true
          break
        case "a":
          this.isPressingA = true
          break
        case "s":
          this.isPressingS = true
          break
        case "d":
          this.isPressingD = true
          break
        case "j":
          this.isPressingJ = true
          break
        case "k":
          this.isPressingK = true
          break
        case "Enter":
          e.preventDefault()
          this.isPressingEnter = true
          break
        case "Tab":
          e.preventDefault()
          this.isPressingTab = true
          break
        case "Space":
          e.preventDefault()
          this.isPressingSpace = true
          break
        case "Shift":
          e.preventDefault()
          this.isPressingShift = true
          break
        case "c":
          this.isPressingC = true
          break
        case "v":
          this.isPressingV = true
          break
      }
    })
  }

  handleJoypadInput() {
    const gamepad = navigator.getGamepads()[0]

    this.emulator.update_input(ButtonEvent.ButtonA, gamepad?.buttons[B_BUTTON].pressed == true || this.isPressingK || this.isPressingSpace)
    this.emulator.update_input(ButtonEvent.ButtonB, gamepad?.buttons[A_BUTTON].pressed == true || this.isPressingJ || this.isPressingShift)
    this.emulator.update_input(ButtonEvent.Select, gamepad?.buttons[SELECT].pressed == true || this.isPressingTab)
    this.emulator.update_input(ButtonEvent.Start, gamepad?.buttons[START].pressed == true || this.isPressingEnter)
    this.emulator.update_input(ButtonEvent.Up, gamepad?.buttons[UP].pressed == true || this.isPressingW)
    this.emulator.update_input(ButtonEvent.Down, gamepad?.buttons[DOWN].pressed == true || this.isPressingS)
    this.emulator.update_input(ButtonEvent.Left, gamepad?.buttons[LEFT].pressed == true || this.isPressingA)
    this.emulator.update_input(ButtonEvent.Right, gamepad?.buttons[RIGHT].pressed == true || this.isPressingD)
    this.emulator.update_input(ButtonEvent.ButtonL, gamepad?.buttons[L_BUTTON].pressed == true || this.isPressingC)
    this.emulator.update_input(ButtonEvent.ButtonR, gamepad?.buttons[R_BUTTON].pressed == true || this.isPressingV)
  }
}
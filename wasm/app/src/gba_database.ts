import { GameStateEntry, StateEntry } from "./game_state_entry"
import moment from "moment"

export class GbaDatabase {
  db: IDBDatabase|null = null
  constructor() {
    const request = indexedDB.open("gba_saves", 4)

    request.onsuccess = (event) => {
      this.db = request.result
    }

    request.onupgradeneeded = (event) => {
      this.db = request.result

      if (!this.db.objectStoreNames.contains("save_states")) {
        this.db.createObjectStore("save_states", { keyPath: "gameName" })
      }
    }

    request.onerror = (event) => {
      console.log('an error occurred while retrieving DB')
    }
  }

  getStateObjectStore() {
    const transaction = this.db?.transaction(["save_states"], "readwrite")

    const objectStore = transaction?.objectStore("save_states")

    return objectStore
  }

  createSaveState(gameName: string, data: Uint8Array, imageUrl: string, stateName: string = "quick_save.state", isUpdate: boolean = false): Promise<StateEntry|null> {
    const objectStore = this.getStateObjectStore()

    const request = objectStore?.get(gameName)

    return new Promise((resolve, reject) => {
      if (request != null) {
        request.onsuccess = (e) => {
          const existing = request.result as GameStateEntry

          if (existing != null) {
            let state = existing.states[stateName]
            let clonedState = null
            if (state == null) {
              state = {
                stateName,
                state: data,
                imageUrl
              }
              existing.states[stateName] = state
            } else {
              state.state = data
              state.imageUrl = imageUrl

              if (isUpdate && state.stateName != "quick_save.state") {
                // "update" the state by removing the old state and naming a new one with a more current name.
                clonedState = { ...state }
                clonedState.stateName = `${moment().unix()}.state`

                delete existing.states[state.stateName]

                existing.states[clonedState.stateName] = clonedState
              }
            }
            objectStore?.put(existing)
            resolve(isUpdate && state.stateName != "quick_save.state" ? clonedState : state)
          } else {
            // create a new state
            const gameStateEntry: GameStateEntry = {
              gameName,
              states: {}
            }

            const state = {
              stateName,
              state: data,
              imageUrl
            }

            gameStateEntry.states[stateName] = state

            objectStore?.put(gameStateEntry)

            resolve(state)
          }
        }

        request.onerror = () => resolve(null)
      } else {
        resolve(null)
      }
    })
  }

  deleteState(gameName: string, stateName: string) {
    const objectStore = this.getStateObjectStore()

    const request = objectStore?.get(gameName)
    return new Promise((resolve, reject) => {
      if (request != null) {
        request.onsuccess = (e) => {


          const entry = request.result as GameStateEntry

          delete(entry.states[stateName])

          objectStore?.put(entry)

          resolve(true)
        }
        request.onerror = () => resolve(false)
      } else {
        resolve(false)
      }
    })
  }

  getSaveStates(gameName: string): Promise<GameStateEntry|null> {
    return new Promise((resolve ,reject) => {
      const objectStore = this.getStateObjectStore()

      const request = objectStore?.get(gameName)

      if (request != null) {
        request.onsuccess = (e) => resolve(request.result as GameStateEntry)
        request.onerror = (e) => resolve(null)
      } else {
        resolve(null)
      }
    })
  }

  loadSaveState(gameName: string, stateName: string = "quick_save.state"): Promise<Uint8Array|null> {
    return new Promise((resolve, reject) => {
      const objectStore = this.getStateObjectStore()

      const request = objectStore?.get(gameName)

      if (request != null) {
        request.onsuccess = (e) => {
          const existing = request.result as GameStateEntry

          if (existing != null) {
            const state = existing.states[stateName]

            resolve(state.state)
          } else {
            resolve(null)
          }
        }
        request.onerror = (e) => resolve(null)
      } else {
        resolve(null)
      }
    })
  }
}
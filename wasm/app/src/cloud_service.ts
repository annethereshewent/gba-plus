import { SaveEntry } from "./save_entry"

const BASE_URL = "https://accounts.google.com/o/oauth2/v2/auth"
const CLIENT_ID = "353451169812-khtanjkfi98eh2bgcldmqt22g47og1ef.apps.googleusercontent.com"

export class CloudService {
  private accessToken: string = ""
  private gbaFolderId: string|null = null
  private expiresIn = 0

  usingCloud = false

  constructor() {
    window.addEventListener("message", (e) => {
      if (e.data == "authFinished") {
        this.getTokenFromStorage()

        const signIn = document.getElementById("cloud-button")

        if (signIn != null) {
          signIn.style.display = "none"
        }

        const isLoggedIn = document.getElementById("cloud-logged-in")

        if (isLoggedIn != null) {
          isLoggedIn.style.display = "block"
        }
      }
    })

    const signIn = document.getElementById("cloud-button")
    const accessToken = localStorage.getItem("gba_access_token")
    this.expiresIn = parseInt(localStorage.getItem("gba_access_expires") || "-1")
    const gbaFolderId = localStorage.getItem("gba_folder_id")

    if (gbaFolderId != null) {
      this.gbaFolderId = gbaFolderId
    }

    if (signIn != null) {
      if (accessToken == null) {
        signIn.addEventListener("click", () => this.oauthSignIn())
      } else if (this.expiresIn == -1 || Date.now() / 1000 < this.expiresIn) {
        this.accessToken = accessToken
        this.usingCloud = true

        signIn.style.display = "none"
        const signOut = document.getElementById("cloud-logged-in")

        if (signOut != null) {
          signOut.style.display = "block"
          signOut.addEventListener("click", () => this.logout())
        }
      } else {
        localStorage.removeItem("gba_access_token")
        localStorage.removeItem("gba_access_expires")
        localStorage.removeItem("gba_folder_id")

        this.silentSignIn()
      }
    }
  }

  async createGbaSavesFolder() {
    if (this.gbaFolderId == null) {
      const params = new URLSearchParams({
        q: `mimeType = "application/vnd.google-apps.folder" and name="gba-saves"`
      })
      const url = `https://www.googleapis.com/drive/v3/files?${params.toString()}`

      const json = await this.cloudRequest(() => fetch(url, {
        headers: {
          Authorization: `Bearer ${this.accessToken}`
        },
      }))

      if (json != null && json.files != null && json.files[0] != null) {
        this.gbaFolderId = json.files[0].id
        localStorage.setItem("gba_folder_id", this.gbaFolderId!!)
      } else {
        // create the folder
        const url = `https://www.googleapis.com/drive/v3/files?uploadType=media`

        const json = await this.cloudRequest(() => fetch(url, {
          method: "POST",
          headers: {
            Authorization: `Bearer ${this.accessToken}`,
            "Content-Type": "application/vnd.google-apps.folder"
          },
          body: JSON.stringify({
            name: "gba-saves",
            mimeType: "application/vnd.google-apps.folder"
          })
        }))


        if (json != null && json.files != null && json.files[0] != null) {
          this.gbaFolderId = json.files[0].id
        }
      }
    }
  }

  getTokenFromStorage() {
    const accessToken = localStorage.getItem("gba_access_token")

    if (accessToken != null) {
      this.accessToken = accessToken
      this.usingCloud = true
    }
  }

  async oauthSignIn() {
    const params = this.getLoginParams()

    const popup = window.open(`${BASE_URL}?${params.toString()}`, "popup", "popup=true,width=650,height=650,resizable=true")

    if (popup != null) {
      let interval = setInterval(() => {
        if (popup.closed) {
          clearInterval(interval)
          location.reload()
        }
      }, 300)
    }
  }

  logout() {
    localStorage.removeItem("gba_access_token")
    localStorage.removeItem("gba_access_expires")
    localStorage.removeItem("gba_user_email")
    localStorage.removeItem("gba_folder_id")

    this.usingCloud = false
    this.accessToken = ""

    const signIn = document.getElementById("cloud-button")

    if (signIn != null) {
      signIn.style.display = "block"

      signIn.addEventListener("click", () => this.oauthSignIn())

      const signOut = document.getElementById("cloud-logged-in")

      if (signOut != null) {
        signOut.style.display = "none"
      }
    }
  }

  silentSignIn() {
    const silentEl = document.getElementById("silent-sign-in") as HTMLIFrameElement

    if (silentEl != null && silentEl.contentWindow != null) {
      const params = this.getLoginParams(true)

      silentEl.contentWindow.window.location.href = `${BASE_URL}?${params.toString()}`
    }
  }

  private async doRequest(request: () => Promise<Response>, returnBuffer: boolean = false): Promise<any> {
    const response = await request()

    if (response.status == 200) {
      const data = returnBuffer ? await response.arrayBuffer() : await response.json()

      return data
    } else {
      localStorage.removeItem("gba_access_token")
      localStorage.removeItem("gba_access_expires")
      localStorage.removeItem("gba_user_email")
      localStorage.removeItem("gba_folder_id")

      this.usingCloud = false
      this.accessToken = ""

      return null
    }
  }

  async cloudRequest(request: () => Promise<Response>, returnBuffer: boolean = false): Promise<any> {
    return new Promise(async (resolve, reject) => {
      if (this.expiresIn < Date.now() / 1000) {
        window.addEventListener("message", async (e) => {
          if (e.data == "authFinished") {
            this.getTokenFromStorage()
            const data = await this.doRequest(request, returnBuffer)

            resolve(data)
          }
        })
        this.silentSignIn()
      } else {
        const data = await this.doRequest(request, returnBuffer)

        resolve(data)
      }
    })
  }

  getLoginParams(noPrompt: boolean = false) {
    // since it always redirects back to the root, location.href should be fine (hopefully!)
    const params = new URLSearchParams({
      client_id: CLIENT_ID,
      redirect_uri: location.href.replace(/\/$/, ''), // remove the trailing slash
      response_type: "token",
      scope: "https://www.googleapis.com/auth/drive.file https://www.googleapis.com/auth/userinfo.email",
    })

    if (noPrompt) {
      const email = localStorage.getItem("gba_user_email")

      if (email != null) {
        params.append("prompt", "none")
        params.append("login_hint", email)
      }

    }

    return params
  }

  async getSaveInfo(gameName: string, searchRoot: boolean = false) {
    await this.createGbaSavesFolder()

    const fileName = gameName.match(/\.sav$/) ? gameName : `${gameName}.sav`


    const query = searchRoot ? `name = "${fileName}"` : `name = "${fileName}" and parents in "${this.gbaFolderId}"`


    const params = new URLSearchParams({
      q: query,
      fields: "files/id,files/parents,files/name"
    })

    const url = `https://www.googleapis.com/drive/v3/files?${params.toString()}`

    return await this.cloudRequest(() => fetch(url, {
      headers: {
        Authorization: `Bearer ${this.accessToken}`
      }
    }))
  }

  async getSave(gameName: string): Promise<SaveEntry> {
    const json = await this.getSaveInfo(gameName)

    if (json != null && json.files != null) {
      const file = json.files[0]

      if (file != null) {

        // retrieve the file data from the cloud
        const url = `https://www.googleapis.com/drive/v3/files/${file.id}?alt=media`

        const body = await this.cloudRequest(() => fetch(url, {
          headers: {
            Authorization: `Bearer ${this.accessToken}`
          }
        }), true)

        return {
          gameName,
          data: new Uint8Array((body as ArrayBuffer))
        }
      }

    }

    return {
      gameName,
      data: new Uint8Array(0)
    }
  }

  async deleteSave(gameName: string): Promise<boolean> {
    const json = await this.getSaveInfo(gameName)

    if (json != null && json.files != null) {
      const url = `https://www.googleapis.com/drive/v3/files/${json.files[0].id}`

      await this.cloudRequest(() => fetch(url, {
        headers: {
          Authorization: `Bearer ${this.accessToken}`
        },
        method: "DELETE"
      }))

      return true
    }

    return false
  }

  async getSaves(): Promise<SaveEntry[]> {
    await this.createGbaSavesFolder()

    const params = new URLSearchParams({
      q: `parents in "${this.gbaFolderId}"`
    })
    const url = `https://www.googleapis.com/drive/v3/files?${params.toString()}`

    const json = await this.cloudRequest(() => fetch(url, {
      headers: {
        Authorization: `Bearer ${this.accessToken}`
      }
    }))

    const saveEntries: SaveEntry[] = []
    if (json != null && json.files != null) {
      for (const file of json.files) {
        saveEntries.push({
          gameName: file.name
        })
      }
    }

    return saveEntries
  }

  async uploadSave(gameName: string, bytes: Uint8Array) {
    const json = await this.getSaveInfo(gameName)

    // this is a hack to get it to change the underlying array buffer
    // (so it doesn't save a bunch of junk from memory unrelated to save)
    const bytesCopy = new Uint8Array(Array.from(bytes))

    const buffer = bytesCopy.buffer

    let resultFile: any
    if (json != null && json.files != null) {
      const file = json.files[0]

      if (file != null) {
        const url = `https://www.googleapis.com/upload/drive/v3/files/${file.id}?uploadType=media`
        await this.cloudRequest(() => fetch(url, {
          method: "PATCH",
          headers: {
            Authorization: `Bearer ${this.accessToken}`,
            "Content-Type": "application/octet-stream",
            "Content-Length": `${bytes.length}`
          },
          body: buffer
        }))
        // there's no need for renaming the file since it's already been uploaded
        return
      } else {
        const url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=media&fields=id,name,parents"
        resultFile = await this.cloudRequest(() => fetch(url, {
          method: "POST",
          headers: {
            Authorization: `Bearer ${this.accessToken}`,
            "Content-Type": "application/octet-stream",
            "Content-Length": `${bytes.length}`
          },
          body: buffer
        }))
      }
    }

    if (resultFile != null) {
      let fileName = !gameName.match(/\.sav$/) ? `${gameName}.sav` : gameName

      const params = new URLSearchParams({
        uploadType: "media",
        addParents: this.gbaFolderId || "",
        removeParents: resultFile.parents.join(",")
      })

      const url = `https://www.googleapis.com/drive/v3/files/${resultFile.id}?${params.toString()}`

      const json = await this.cloudRequest(() => fetch(url, {
        method: "PATCH",
        headers: {
          Authorization: `Bearer ${this.accessToken}`,
          "Content-Type": "application/octet-stream"
        },
        body: JSON.stringify({
          name: fileName,
          mimeType: "application/octet-stream"
        })
      }))
    }
  }

  async getLoggedInEmail() {
    const url = "https://www.googleapis.com/oauth2/v2/userinfo"

    const json = await this.cloudRequest(() => fetch(url, {
      headers: {
        Authorization: `Bearer ${this.accessToken}`
      }
    }))

    if (json != null && json.email != null) {
      localStorage.setItem("gba_user_email", json.email)
    }
  }

  async checkAuthentication() {
    if (window.location.href.indexOf("#") != -1) {
      const tokenParams = window.location.href.split("#")[1].split("&")

      let accessToken = tokenParams.filter((param) => param.indexOf('access_token') != -1)[0]
      let expires = tokenParams.filter((param) => param.indexOf('expires_in') != -1)[0]

      if (accessToken != null) {
        accessToken = accessToken.split("=")[1]

        if (expires != null) {
          expires = expires.split("=")[1]

          const timestamp = parseInt(expires) + Date.now() / 1000

          localStorage.setItem("gba_access_expires", timestamp.toString())

          this.expiresIn = timestamp
        }

        localStorage.setItem("gba_access_token", accessToken)

        this.accessToken = accessToken
        this.usingCloud = true

        // finally get logged in user email
        await this.getLoggedInEmail()

        parent.postMessage("authFinished", "*")

        window.close()
      }
    }

  }
}
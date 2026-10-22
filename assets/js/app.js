import {Socket} from "../../deps/phoenix/priv/static/phoenix.mjs"
import {LiveSocket} from "../../deps/phoenix_live_view/priv/static/phoenix_live_view.esm.js"
const csrfToken = document.querySelector("meta[name='csrf-token']").getAttribute("content")
const PickerFocus = {
  mounted() { this.step = this.el.dataset.step },
  updated() {
    const step = this.el.dataset.step
    if (step === this.step) return
    const nextRound = step === "book" && this.step === "result"
    this.step = step
    const target = nextRound ? document.querySelector("#verse-text") : this.el.querySelector("#picker-heading")
    target.focus({preventScroll: true})
    if (window.matchMedia("(max-width: 760px)").matches) {
      const behavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth"
      const panel = nextRound ? target.closest(".verse-panel") : this.el
      panel.scrollIntoView({behavior, block: "start"})
    }
  }
}
const AtlasReader = {
  mounted() {
    this.previousFocus = document.activeElement
    this.previousOverflow = document.body.style.overflow
    document.body.style.overflow = "hidden"
    this.el.querySelector("#atlas-reader-close").focus()
    this.onMatch = () => {
      const match = this.el.querySelector(".atlas-reading-verse.matched,.atlas-reading-heading.matched")
      const behavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth"
      if (match) match.scrollIntoView({behavior, block: "center"})
    }
    this.el.addEventListener("atlas:first-match", this.onMatch)
  },
  destroyed() {
    document.body.style.overflow = this.previousOverflow
    this.el.removeEventListener("atlas:first-match", this.onMatch)
    if (this.previousFocus?.isConnected) this.previousFocus.focus({preventScroll: true})
  }
}
const AtlasFilters = {
  mounted() {
    this.media = window.matchMedia("(max-width: 840px)")
    this.onResize = () => this.sync()
    this.onKey = event => {
      if (!this.open) return
      if (event.key === "Escape") { event.preventDefault(); this.pushEvent("close-filters", {}) }
      if (event.key === "Tab") {
        const controls = [...this.el.querySelectorAll("button,input,summary,a")].filter(el => el.offsetParent !== null && !el.disabled)
        const first = controls[0], last = controls.at(-1)
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus() }
        if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus() }
      }
    }
    this.media.addEventListener("change", this.onResize)
    document.addEventListener("keydown", this.onKey)
    this.sync()
  },
  updated() { this.sync() },
  sync() {
    const open = this.el.dataset.open === "true" && this.media.matches
    if (open) {
      this.el.setAttribute("role", "dialog")
      this.el.setAttribute("aria-modal", "true")
      this.el.setAttribute("aria-label", "Explore the Atlas")
      this.background?.forEach(el => el.inert = true)
    }
    if (open === !!this.open) return
    this.open = open
    if (open) {
      this.previousFocus = document.activeElement
      this.previousOverflow = document.body.style.overflow
      document.body.style.overflow = "hidden"
      this.el.setAttribute("role", "dialog")
      this.el.setAttribute("aria-modal", "true")
      this.el.setAttribute("aria-label", "Explore the Atlas")
      this.background = [...document.querySelectorAll(".site-header,.atlas-map-panel,footer")]
      this.background.forEach(el => el.inert = true)
      this.el.querySelector("#atlas-search-input").focus()
    } else this.release()
  },
  release() {
    document.body.style.overflow = this.previousOverflow || ""
    this.el.removeAttribute("role")
    this.el.removeAttribute("aria-modal")
    this.el.removeAttribute("aria-label")
    this.background?.forEach(el => el.inert = false)
    if (this.previousFocus?.isConnected) this.previousFocus.focus({preventScroll: true})
  },
  destroyed() {
    if (this.open) this.release()
    this.media.removeEventListener("change", this.onResize)
    document.removeEventListener("keydown", this.onKey)
  }
}
const liveSocket = new LiveSocket("/live", Socket, {
  params: {_csrf_token: csrfToken},
  hooks: {PickerFocus, AtlasReader, AtlasFilters}
})
liveSocket.connect()
window.liveSocket = liveSocket

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
const liveSocket = new LiveSocket("/live", Socket, {
  params: {_csrf_token: csrfToken},
  hooks: {PickerFocus}
})
liveSocket.connect()
window.liveSocket = liveSocket

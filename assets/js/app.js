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
const ChapterReader = {
  mounted() {
    this.previousFocus = document.activeElement
    this.previousFocusId = this.previousFocus?.id
    this.previousOverflow = document.body.style.overflow
    document.body.style.overflow = "hidden"
    this.focusReader = () => {
      if (!this.el.contains(document.activeElement)) {
        this.el.querySelector("[data-reader-close]").focus({preventScroll: true})
      }
    }
    this.focusReader()
    this.onMatch = () => {
      const match = this.el.querySelector(".atlas-reading-verse.matched,.atlas-reading-heading.matched")
      const behavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth"
      if (match) match.scrollIntoView({behavior, block: "center"})
    }
    this.el.addEventListener("chapter:first-match", this.onMatch)
    this.updated()
  },
  updated() {
    const ready = this.el.dataset.ready === "true"
    if (ready && !this.ready && this.el.dataset.autoScroll === "true") this.onMatch()
    this.ready = ready
    this.focusReader()
  },
  destroyed() {
    document.body.style.overflow = this.previousOverflow
    this.el.removeEventListener("chapter:first-match", this.onMatch)
    const previousFocus = this.previousFocus?.isConnected ? this.previousFocus : document.getElementById(this.previousFocusId)
    previousFocus?.focus({preventScroll: true})
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
// Native disclosures work on controller pages and LiveViews alike.
document.addEventListener("click", event => {
  document.querySelectorAll(".account-menu[open]").forEach(menu => {
    if (!menu.contains(event.target)) menu.open = false
  })
})
document.addEventListener("keydown", event => {
  if (event.key !== "Escape") return
  document.querySelectorAll(".account-menu[open]").forEach(menu => {
    menu.open = false
    menu.querySelector("summary").focus()
    event.preventDefault()
  })
})

const meterStates = new WeakMap()
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)")
function animateMeters() {
  document.querySelectorAll("[data-meter]").forEach(meter => {
    const bar = meter.querySelector("[data-meter-fill]")
    if (!bar) return
    const value = Math.max(0, Math.min(100, Number(meter.dataset.meter) || 0))
    const stage = meter.dataset.meterStage
    const previous = meterStates.get(meter)
    if (previous?.value === value && previous.stage === stage && previous.bar === bar) return
    let from = previous?.stage === stage && previous.bar === bar ? previous.value : 0
    if (previous?.animation?.playState === "running" && previous.stage === stage && previous.bar === bar) {
      from = 100 * bar.getBoundingClientRect().width / Math.max(1, meter.clientWidth)
    }
    previous?.animation?.cancel()
    const animation = reducedMotion.matches || from === value ? null : bar.animate(
      [{width: `${from}%`}, {width: `${value}%`}],
      {duration: 650, easing: "cubic-bezier(.2,.7,.2,1)"}
    )
    meterStates.set(meter, {value, stage, bar, animation})
  })
}
reducedMotion.addEventListener("change", () => {
  if (reducedMotion.matches) document.querySelectorAll("[data-meter]").forEach(meter => meterStates.get(meter)?.animation?.cancel())
})
document.addEventListener("phx:update", animateMeters)
animateMeters()

const CopyResult = {
  mounted() {
    this.copy = async () => {
      const result = document.getElementById("daily-result")
      const status = document.getElementById("copy-daily-status")
      try {
        await navigator.clipboard.writeText(result.value)
        status.textContent = "Copied!"
      } catch {
        result.focus()
        result.select()
        status.textContent = "Selected — copy the text above."
      }
    }
    this.el.addEventListener("click", this.copy)
  },
  destroyed() { this.el.removeEventListener("click", this.copy) }
}

const CopyText = {
  mounted() {
    this.copy = async () => {
      const target = document.getElementById(this.el.dataset.copyTarget)
      const status = document.getElementById(this.el.dataset.copyStatus)
      try {
        await navigator.clipboard.writeText(target.value)
        status.textContent = "Copied!"
      } catch {
        target.focus()
        target.select()
        status.textContent = "Selected — copy the link above."
      }
    }
    this.el.addEventListener("click", this.copy)
  },
  destroyed() { this.el.removeEventListener("click", this.copy) }
}

const InviteQR = {
  mounted() {
    this.previousFocus = document.activeElement
    this.previousOverflow = document.body.style.overflow
    document.body.style.overflow = "hidden"
    this.cancel = event => { event.preventDefault(); this.pushEvent("close-invite", {}) }
    this.outside = event => {
      const rect = this.el.getBoundingClientRect()
      if (event.target === this.el && (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom)) {
        this.pushEvent("close-invite", {})
      }
    }
    this.el.addEventListener("cancel", this.cancel)
    this.el.addEventListener("click", this.outside)
    this.el.showModal()
  },
  destroyed() {
    this.el.close()
    document.body.style.overflow = this.previousOverflow
    this.el.removeEventListener("cancel", this.cancel)
    this.el.removeEventListener("click", this.outside)
    if (this.previousFocus?.isConnected) this.previousFocus.focus({preventScroll: true})
  }
}

const MatchCountdown = {
  mounted() {
    this.sync()
    this.timer = setInterval(() => this.renderTime(), 250)
  },
  updated() { this.sync() },
  sync() {
    const key = `${this.el.dataset.deadline}/${this.el.dataset.serverNow}`
    if (key === this.key) return
    this.key = key
    // Start from server time so an inaccurate device clock cannot skew the display.
    this.remaining = Date.parse(this.el.dataset.deadline) - Date.parse(this.el.dataset.serverNow)
    this.started = performance.now()
    this.renderTime()
  },
  renderTime() {
    const seconds = Math.max(0, Math.ceil((this.remaining - (performance.now() - this.started)) / 1000))
    const text = `${seconds}s`
    if (this.el.textContent !== text) this.el.textContent = text
  },
  destroyed() { clearInterval(this.timer) }
}

const MatchRound = {
  mounted() { this.phase = this.el.dataset.phase; this.round = this.el.dataset.round },
  updated() {
    const phase = this.el.dataset.phase, round = this.el.dataset.round
    if (phase === this.phase && round === this.round) return
    this.phase = phase; this.round = round
    const selector = phase === "round" ? "#verse-text" : phase === "reveal" ? "#match-round-result" : ".match-heading h1"
    const target = this.el.querySelector(selector)
    if (!target) return
    target.focus({preventScroll: true})
    if (window.matchMedia("(max-width: 760px)").matches) {
      const behavior = window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "instant" : "smooth"
      const panel = target.closest(".verse-panel") || target
      panel.scrollIntoView({behavior, block: "center"})
    }
  }
}

const HomeGames = {
  mounted() {
    this.onVisible = () => {
      if (document.visibilityState === "visible") this.pushEvent("refresh-home", {})
    }
    document.addEventListener("visibilitychange", this.onVisible)
  },
  destroyed() { document.removeEventListener("visibilitychange", this.onVisible) }
}

const liveSocket = new LiveSocket("/live", Socket, {
  params: {_csrf_token: csrfToken},
  hooks: {PickerFocus, ChapterReader, AtlasFilters, CopyResult, CopyText, InviteQR, MatchRound, MatchCountdown, HomeGames}
})
liveSocket.connect()
window.liveSocket = liveSocket

const clocks = new WeakMap()

export function formatDeadline(deadline, locale, timeZone) {
  return new Intl.DateTimeFormat(locale, {
    month: "short", day: "numeric", hour: "numeric", minute: "2-digit",
    timeZoneName: "short", ...(timeZone ? {timeZone} : {})
  }).format(deadline)
}

export function formatRemaining(milliseconds) {
  if (milliseconds <= 0) return "Reset reached"
  const minutes = Math.floor(milliseconds / 60000)
  if (minutes === 0) return "Less than a minute left"
  if (minutes < 60) return `${minutes}m left`
  return `${Math.floor(minutes / 60)}h ${minutes % 60}m left`
}

export function updateDailyDeadlines(root = document) {
  const elapsedNow = performance.now()
  root.querySelectorAll("[data-daily-deadline]").forEach(element => {
    const {deadline, serverNow, label} = element.dataset
    const key = `${deadline}/${serverNow}`
    let clock = clocks.get(element)
    if (!clock || clock.key !== key) {
      const date = new Date(deadline)
      const remaining = date.getTime() - Date.parse(serverNow)
      if (!Number.isFinite(remaining)) return
      clock = {key, date, remaining, started: elapsedNow}
      clocks.set(element, clock)
    }
    const localTime = element.querySelector("[data-deadline-time]")
    const countdown = element.querySelector("[data-deadline-countdown]")
    // Reapply after LiveView patches, which may restore the server's UTC fallback.
    const text = `${label} ${formatDeadline(clock.date)} (your time)`
    if (localTime.textContent !== text) localTime.textContent = text
    if (countdown) {
      // The server clock anchors the countdown, even if the device clock is wrong.
      const text = ` · ${formatRemaining(clock.remaining - (elapsedNow - clock.started))}`
      if (countdown.textContent !== text) countdown.textContent = text
    }
  })
}

import assert from "node:assert/strict"
import {test} from "node:test"
import {formatDeadline, formatRemaining, updateDailyDeadlines} from "../../assets/js/daily_deadline.mjs"

test("the same UTC deadline displays on the correct local date across timezones", () => {
  const deadline = new Date("2026-11-09T00:00:00Z")
  const boise = formatDeadline(deadline, "en-US", "America/Boise")
  assert.match(boise, /Nov 8/)
  assert.match(boise, /5:00 PM/)
  assert.match(boise, /MST/)
  const auckland = formatDeadline(deadline, "en-US", "Pacific/Auckland")
  assert.match(auckland, /Nov 9/)
  assert.match(auckland, /1:00 PM/)
})

test("the local deadline follows daylight saving for the actual deadline date", () => {
  assert.match(formatDeadline(new Date("2026-11-01T00:00:00Z"), "en-US", "America/Boise"), /6:00 PM MDT/)
  assert.match(formatDeadline(new Date("2026-11-02T00:00:00Z"), "en-US", "America/Boise"), /5:00 PM MST/)
})

function fixture({countdown = true} = {}) {
  const time = {textContent: "Finish by Nov 9 at 00:00 UTC"}
  const remaining = {textContent: ""}
  const element = {
    dataset: {deadline: "2026-11-09T00:00:00Z", serverNow: "2026-11-08T22:00:00Z", label: "Finish by"},
    querySelector: selector => selector === "[data-deadline-time]" ? time : countdown ? remaining : null
  }
  return {element, time, remaining, root: {querySelectorAll: () => [element]}}
}

test("countdown uses server time, survives patches, and refreshes its server anchor", t => {
  let elapsed = 0
  t.mock.method(performance, "now", () => elapsed)
  t.mock.method(Date, "now", () => Date.parse("2040-01-01T00:00:00Z"))
  const view = fixture()
  updateDailyDeadlines(view.root)
  assert.equal(view.remaining.textContent, " · 2h 0m left")
  assert.match(view.time.textContent, /^Finish by .*\(your time\)$/)

  elapsed = 90_000
  view.time.textContent = "UTC fallback restored by a patch"
  updateDailyDeadlines(view.root)
  assert.equal(view.remaining.textContent, " · 1h 58m left")
  assert.match(view.time.textContent, /\(your time\)$/)

  view.element.dataset.serverNow = "2026-11-08T23:59:00Z"
  updateDailyDeadlines(view.root)
  assert.equal(view.remaining.textContent, " · 1m left")
  elapsed += 60_000
  updateDailyDeadlines(view.root)
  assert.equal(view.remaining.textContent, " · Reset reached")
})

test("expired deadlines can show their local timestamp without a live countdown", () => {
  const view = fixture({countdown: false})
  view.element.dataset.label = "Closed"
  updateDailyDeadlines(view.root)
  assert.match(view.time.textContent, /^Closed .*\(your time\)$/)
  assert.equal(view.remaining.textContent, "")
})

test("missing or invalid timing data leaves the readable UTC fallback intact", () => {
  const view = fixture()
  view.element.dataset.serverNow = "invalid"
  updateDailyDeadlines(view.root)
  assert.equal(view.time.textContent, "Finish by Nov 9 at 00:00 UTC")
})

test("remaining time stays positive until the actual cutoff and never becomes negative", () => {
  assert.equal(formatRemaining(1), "Less than a minute left")
  assert.equal(formatRemaining(60_001), "1m left")
  assert.equal(formatRemaining(0), "Reset reached")
  assert.equal(formatRemaining(-1000), "Reset reached")
})

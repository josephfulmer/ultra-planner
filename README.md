# Ridge — 50K Training Plan Generator

A small Rust web app that turns four numbers about your current fitness into a
full week-by-week training plan for a 50K ultra, and lets you export it
straight into Apple Calendar (or Google Calendar / Outlook — anything that
reads `.ics`).

Backend: Rust + [Axum](https://github.com/tokio-rs/axum).
Frontend: plain HTML/CSS/JS served as static files (no build step, no
framework) — trail-themed dark UI with an "elevation profile" chart of your
mileage ramp.

## Running it

You need Rust installed ([rustup.rs](https://rustup.rs) if you don't have it,
Rust 1.75+ is fine).

```bash
cd ultra-planner
cargo run
```

Then open **http://localhost:3000** in your browser.

## How the plan is built

- **Phases**: Base → Build → Peak → Taper, roughly 40/35/15/10% of the
  weeks available, with a minimum 2-week taper.
- **Weekly volume** ramps from your current weekly km up to a peak based on
  your experience level (beginner/intermediate/advanced), with a cutback
  (recovery) week roughly every 4th week.
- **Long runs** ramp from your current longest run up toward a peak long run,
  and in the last stretch of Build and all of Peak, back-to-back weekend
  long runs are introduced (Saturday medium-long + Sunday long) — this is
  standard ultra-specific "time on tired legs" training.
- **Taper** cuts volume progressively, ending in a very light race week with
  a short shake-out run before race day.
- **Training days**: Sunday is always the long run day; which other days get
  used depends on how many days/week you selected (3–6), with Thursday as
  the recurring quality session (hills in Base, tempo in Build/Peak).

This is a heuristic, not a certified coaching plan — it's meant as a solid
default structure, not a replacement for a coach, especially if you have an
injury history or a specific goal time. If a week's plan feels like too
much, it's fine to repeat a cutback week or drop a day.

## Project layout

```
ultra-planner/
├── Cargo.toml
├── src/
│   ├── main.rs      # Axum server + routes
│   ├── plan.rs       # Training plan generation algorithm
│   └── ics.rs         # .ics (iCalendar) export
└── static/
    ├── index.html
    ├── style.css
    └── app.js
```

## API

- `POST /api/generate` — takes the athlete inputs as JSON, returns the full
  plan as JSON.
- `POST /api/export-ics` — same input, returns a `.ics` file you can import
  into any calendar app.

Example input:
```json
{
  "current_weekly_km": 30,
  "longest_recent_run_km": 16,
  "experience": "intermediate",
  "days_per_week": 4,
  "race_date": "2026-12-01"
}
```
`experience` is one of `"beginner"`, `"intermediate"`, `"advanced"`.

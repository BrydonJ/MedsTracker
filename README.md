# Meds Tracker

A fast, private app for packing a weekly morning/night pill organiser and keeping on top of what to order. It replaces the printed checklist.

Built in Rust: [Leptos](https://leptos.dev) compiled to WebAssembly, with a packing engine that has no UI dependencies and is covered by unit tests. Data never leaves the device (browser `localStorage`), and the app works offline once installed.

## Features

- **Morning row, then night row.** Packing goes along the organiser one row at a time, one medicine at a time. The card in front of you shows the dose in big type, which days get it, and any halving, with one big **Packed** button. The rest of the row sits underneath; tap one to open it, or tick it straight off. Morning and night each have their own progress, the night row starts by itself once the morning is done, and packed medicines collapse into chips you can tap to undo. Built for a phone first.
- **Half tablets handled for you.** The app works out how many tablets to cut and remembers the spare half for next week.
- **Weekly or specific-day medicines** show a mini organiser grid so they land in the right compartment.
- **Check organiser:** a 7 × 2 grid of how many pieces should be in every compartment. Tap one to see its contents.
- **Ordering:** tap "Running low?" while packing to flag a medicine. Medicines with a stock count are flagged automatically when they drop under a threshold you choose. Send the list to the pharmacy from the phone's share sheet, mark items as ordered, and add the tablets to stock when they arrive.
- **Stock (optional):** finishing a week subtracts what you packed, and the app warns you before you're short.
- **Short on stock?** The card shows which days to pack as a row of day toggles. Tap days on or off, or tap **Fit to stock** to fill the earliest days the box covers. Only those compartments are counted, and the week's history notes the days left empty.
- **Add a delivery:** one sheet for everything that came in. Type a count or tap **Box of N** for each medicine, then add it all to stock at once (with undo). Open it from the Order tab, the start screen or a short card.
- **Pause** a medicine instead of deleting it.
- **Print** a packing checklist laid out like the paper sheet.
- **Backup / restore** as a JSON file, light/dark theme, installable as a PWA.

## Development

```sh
cargo test -p meds-core          # packing engine tests
cd app && trunk serve --open     # dev server
cd app && trunk build --release  # production build in app/dist
```

Needs the `wasm32-unknown-unknown` target and [trunk](https://trunkrs.dev).

Pushes to `main` run the tests and deploy `app/dist` to GitHub Pages (`.github/workflows/deploy.yml`).

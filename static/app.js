const KM_PER_MI = 1.60934;

// ---------- Unit toggles ----------
const unitState = { volume: 'km', longrun: 'km' };

document.querySelectorAll('.unit-toggle').forEach((toggle) => {
  const target = toggle.dataset.target;
  const input = toggle.previousElementSibling;
  toggle.querySelectorAll('.unit-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const newUnit = btn.dataset.unit;
      if (newUnit === unitState[target]) return;
      const val = parseFloat(input.value);
      if (!isNaN(val)) {
        input.value = newUnit === 'mi'
          ? Math.round((val / KM_PER_MI) * 10) / 10
          : Math.round(val * KM_PER_MI * 10) / 10;
      }
      unitState[target] = newUnit;
      toggle.querySelectorAll('.unit-btn').forEach((b) => b.classList.toggle('active', b === btn));
    });
  });
});

function toKm(value, unitKey) {
  const v = parseFloat(value) || 0;
  return unitState[unitKey] === 'mi' ? v * KM_PER_MI : v;
}

// Default race date: 18 weeks out, a common minimum ultra build.
const raceDateInput = document.getElementById('raceDate');
const defaultRace = new Date();
defaultRace.setDate(defaultRace.getDate() + 18 * 7);
raceDateInput.value = defaultRace.toISOString().slice(0, 10);
raceDateInput.min = new Date(Date.now() + 6 * 7 * 86400000).toISOString().slice(0, 10);

// ---------- Form submit ----------
const form = document.getElementById('planForm');
const formError = document.getElementById('formError');
const briefing = document.getElementById('briefing');
const results = document.getElementById('results');
let lastInput = null;
let lastPlan = null;

form.addEventListener('submit', async (e) => {
  e.preventDefault();
  formError.hidden = true;

  const input = {
    current_weekly_km: Math.round(toKm(document.getElementById('currentVolume').value, 'volume') * 10) / 10,
    longest_recent_run_km: Math.round(toKm(document.getElementById('longestRun').value, 'longrun') * 10) / 10,
    experience: document.getElementById('experience').value,
    days_per_week: parseInt(document.getElementById('daysPerWeek').value, 10),
    race_date: raceDateInput.value,
  };

  const submitBtn = form.querySelector('.cta');
  submitBtn.textContent = 'Charting…';
  submitBtn.disabled = true;

  try {
    const res = await fetch('/api/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });
    if (!res.ok) throw new Error(await res.text());
    const plan = await res.json();
    lastInput = input;
    lastPlan = plan;
    renderResults(plan);
    briefing.hidden = true;
    results.hidden = false;
    window.scrollTo({ top: 0, behavior: 'smooth' });
  } catch (err) {
    formError.textContent = "Couldn't chart that route — double check your inputs and try again.";
    formError.hidden = false;
    console.error(err);
  } finally {
    submitBtn.textContent = 'Chart the route';
    submitBtn.disabled = false;
  }
});

document.getElementById('startOver').addEventListener('click', () => {
  results.hidden = true;
  briefing.hidden = false;
  window.scrollTo({ top: 0, behavior: 'smooth' });
});

document.getElementById('downloadIcs').addEventListener('click', async () => {
  if (!lastInput) return;
  const btn = document.getElementById('downloadIcs');
  btn.textContent = 'Preparing…';
  btn.disabled = true;
  try {
    const res = await fetch('/api/export-ics', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(lastInput),
    });
    if (!res.ok) throw new Error(await res.text());
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = '50k-training-plan.ics';
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  } catch (err) {
    console.error(err);
    alert("Couldn't build the calendar file. Try again in a moment.");
  } finally {
    btn.textContent = 'Add to Apple Calendar';
    btn.disabled = false;
  }
});

// ---------- Rendering ----------
function fmtDate(iso) {
  const d = new Date(iso + 'T00:00:00');
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function phaseColor(phase) {
  return { Base: '#566451', Build: '#7c9473', Peak: '#e2622b', Taper: '#7c8272' }[phase] || '#7c8272';
}

function renderResults(plan) {
  const raceDateStr = new Date(plan.race_date + 'T00:00:00')
    .toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });
  document.getElementById('resultsTitle').textContent = `The climb to race day \u2014 ${raceDateStr}`;

  const warningBanner = document.getElementById('warningBanner');
  if (plan.compressed_warning) {
    warningBanner.textContent = plan.compressed_warning;
    warningBanner.hidden = false;
  } else {
    warningBanner.hidden = true;
  }

  document.getElementById('statStrip').innerHTML = `
    <div class="stat"><p class="stat-label">Weeks</p><p class="stat-value">${plan.total_weeks}</p></div>
    <div class="stat"><p class="stat-label">Peak week</p><p class="stat-value">${plan.peak_weekly_km.toFixed(0)} km</p></div>
    <div class="stat"><p class="stat-label">Longest run</p><p class="stat-value">${plan.peak_long_run_km.toFixed(0)} km</p></div>
    <div class="stat"><p class="stat-label">Race distance</p><p class="stat-value">50 km</p></div>
  `;

  renderChart(plan);
  renderWeeks(plan);
}

function renderChart(plan) {
  const svg = document.getElementById('profileChart');
  const weeks = plan.weeks;
  const W = 1000, H = 260;
  const padTop = 20, padBottom = 30, padX = 10;
  const maxKm = Math.max(...weeks.map((w) => w.target_km)) * 1.08;
  const minKm = 0;

  const n = weeks.length;
  const xStep = n > 1 ? (W - padX * 2) / (n - 1) : 0;
  const points = weeks.map((w, i) => {
    const x = padX + i * xStep;
    const y = padTop + (1 - (w.target_km - minKm) / (maxKm - minKm)) * (H - padTop - padBottom);
    return { x, y, phase: w.phase, week: w };
  });

  // Gradient with hard color stops per phase, so the single stroke reads as segmented.
  const stops = points.map((p, i) => {
    const offset = n > 1 ? (i / (n - 1)) * 100 : 0;
    return `<stop offset="${offset}%" stop-color="${phaseColor(p.phase)}" />`;
  }).join('');

  const linePath = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x.toFixed(1)} ${p.y.toFixed(1)}`).join(' ');
  const areaPath = `${linePath} L ${points[points.length - 1].x.toFixed(1)} ${H - padBottom} L ${points[0].x.toFixed(1)} ${H - padBottom} Z`;

  const dots = points.map((p) => `
    <circle cx="${p.x.toFixed(1)}" cy="${p.y.toFixed(1)}" r="3.5" fill="${phaseColor(p.phase)}">
      <title>Week ${p.week.week_number} \u2014 ${p.phase} \u2014 ${p.week.target_km.toFixed(0)} km</title>
    </circle>`).join('');

  const lastPt = points[points.length - 1];

  svg.innerHTML = `
    <defs>
      <linearGradient id="ridgeGradient" x1="0" y1="0" x2="1" y2="0">
        ${stops}
      </linearGradient>
      <linearGradient id="ridgeFade" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#e2622b" stop-opacity="0.22" />
        <stop offset="100%" stop-color="#e2622b" stop-opacity="0" />
      </linearGradient>
    </defs>
    <line x1="${padX}" y1="${H - padBottom}" x2="${W - padX}" y2="${H - padBottom}" stroke="#33382a" stroke-width="1" />
    <path d="${areaPath}" fill="url(#ridgeFade)" />
    <path d="${linePath}" fill="none" stroke="url(#ridgeGradient)" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round" />
    ${dots}
    <text x="${lastPt.x - 4}" y="${lastPt.y - 12}" text-anchor="end" font-family="IBM Plex Mono" font-size="11" fill="#ede9df">50K \u25b2</text>
  `;
}

function renderWeeks(plan) {
  const container = document.getElementById('weekList');
  container.innerHTML = '';

  plan.weeks.forEach((week) => {
    const card = document.createElement('div');
    card.className = 'week-card';

    const runDays = week.days.filter((d) => d.distance_km && d.distance_km > 0 && d.kind !== 'RaceDay').length;
    const raceThisWeek = week.days.some((d) => d.kind === 'RaceDay');

    card.innerHTML = `
      <div class="week-card-head">
        <span class="week-num">WK ${String(week.week_number).padStart(2, '0')}</span>
        <span class="week-phase phase-${week.phase}">${week.phase}</span>
        <span class="week-dates">${fmtDate(week.start_date)} \u2013 ${fmtDate(addDays(week.start_date, 6))}${raceThisWeek ? ' \u2014 race week' : ''}</span>
        <span class="week-summary">${week.target_km.toFixed(0)} km<small>${runDays} runs \u00b7 long ${week.long_run_km.toFixed(0)} km</small></span>
        <span class="week-caret">\u25b6</span>
      </div>
      <div class="week-days">
        ${week.days.map(dayChip).join('')}
      </div>
    `;

    card.querySelector('.week-card-head').addEventListener('click', () => {
      card.classList.toggle('open');
    });

    container.appendChild(card);
  });

  // Open the first week by default so results don't look empty.
  const first = container.querySelector('.week-card');
  if (first) first.classList.add('open');
}

function dayChip(day) {
  const isRest = day.kind === 'Rest';
  const isRace = day.kind === 'RaceDay';
  const dist = day.distance_km ? `${day.distance_km.toFixed(0)} km` : '';
  return `
    <div class="day-chip ${isRest ? 'rest' : ''} ${isRace ? 'race' : ''}">
      <p class="day-weekday">${day.weekday}</p>
      <p class="day-kind">${labelFor(day)}</p>
      ${dist ? `<p class="day-dist">${dist}</p>` : ''}
      <p class="day-notes">${day.notes}</p>
    </div>
  `;
}

const KIND_LABELS = {
  Rest: 'Rest',
  Easy: 'Easy run',
  MediumLong: 'Medium-long',
  Long: 'Long run',
  BackToBackLong: 'B2B long run',
  Tempo: 'Tempo',
  Hills: 'Hill repeats',
  Strides: 'Easy + strides',
  CrossTrain: 'Cross-train',
  Strength: 'Strength',
  ShakeOut: 'Shake-out',
  RaceDay: 'RACE DAY',
};
function labelFor(day) {
  return KIND_LABELS[day.kind] || day.label;
}

function addDays(iso, n) {
  const d = new Date(iso + 'T00:00:00');
  d.setDate(d.getDate() + n);
  return d.toISOString().slice(0, 10);
}

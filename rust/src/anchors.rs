//! Literature anchors. Each function reproduces one published quantity from the model so that
//! `metrics.json` and the tests report the same number. Sources: `data/SOURCES.md`.

use crate::hormones::*;

/// First-hour gastric emptying with the agonist divided by without (day 3, 08:00 meal).
/// Hjerpsted 2018 reports 0.73 (95% CI 0.61 to 0.87) for semaglutide.
pub fn first_hour_emptying_ratio(p: &Params) -> f64 {
    let emptied = |dose: bool| {
        let mut day = Day::standard();
        if dose {
            day.doses = vec![Dose {
                at_h: 8.0,
                units: 1.0,
            }];
        }
        let tr = run(p, &day, 72.0, 10.0);
        let i0 = ((48.0 + 8.0) * HOUR / 10.0) as usize + 1;
        let i1 = ((48.0 + 9.0) * HOUR / 10.0) as usize;
        tr.s[i0].stomach - tr.s[i1].stomach
    };
    emptied(true) / emptied(false)
}

/// Mean glucose in the 3 h after each meal at 12 h of circadian misalignment divided by aligned.
/// Morris 2015 reports +6% (ratio 1.06).
pub fn misalign_glucose_ratio(p: &Params) -> f64 {
    let post = |shift: f64| {
        let day = Day::standard();
        let mut s = State::basal(p);
        let e = Ext {
            circ_shift_h: shift,
            ..Ext::default()
        };
        let (mut sum, mut n) = (0.0, 0usize);
        for k in 0..(72 * 360) {
            let h = k as f64 * 10.0 / HOUR;
            apply_events(&mut s, &day, h, 10.0);
            step_ext(&mut s, p, h, 10.0, day.sleep_h, &e);
            let hc = h.rem_euclid(24.0);
            let in_post = (8.0..11.0).contains(&hc)
                || (12.5..15.5).contains(&hc)
                || (18.5..21.5).contains(&hc);
            if h >= 48.0 && in_post {
                sum += s.glucose;
                n += 1;
            }
        }
        sum / n as f64
    };
    post(12.0) / post(0.0)
}

/// Minutes from waking (07:00) to the morning cortisol peak. Stalder 2016: 30 to 45 min.
pub fn car_peak_minutes(p: &Params) -> f64 {
    let tr = run(p, &Day::standard(), 48.0, 60.0);
    let (mut best, mut at) = (0.0, 0.0);
    for (s, h) in tr.s.iter().zip(&tr.h) {
        if *h >= 29.0 && *h < 36.0 && s.cortisol > best {
            best = s.cortisol;
            at = *h;
        }
    }
    (at - 31.0) * 60.0
}

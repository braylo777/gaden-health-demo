use gaden::fit::*;
use gaden::hormones::Params;

#[test]
fn targets_csv_parses() {
    assert_eq!(targets().len(), 9);
}

#[test]
fn targets_have_known_sources() {
    for t in targets() {
        assert!(
            ["cummings2001", "sinha1996", "elliott1993", "spiegel2004"]
                .contains(&t.source.as_str()),
            "{}",
            t.source
        );
    }
}

#[test]
fn targets_have_positive_scale() {
    assert!(targets().iter().all(|t| t.scale > 0.0));
}

#[test]
fn every_target_has_a_model_summary() {
    let s = summaries(&Params::default());
    for t in targets() {
        assert!(s.iter().any(|(k, _)| *k == t.id), "{}", t.id);
    }
}

#[test]
fn sources_file_lists_every_source() {
    let md = include_str!("../../data/SOURCES.md");
    for t in targets() {
        assert!(md.contains(&t.source), "{}", t.source);
    }
}

#[test]
fn parse_rejects_bad_number() {
    let csv = "h\nx,q,abc,u,1,point,s,\"q\"";
    assert!(parse_targets(csv).is_err());
}

#[test]
fn parse_rejects_zero_scale() {
    let csv = "h\nx,q,1,u,0,point,s,\"q\"";
    assert!(parse_targets(csv).is_err());
}

#[test]
fn parse_rejects_short_row() {
    assert!(parse_targets("h\nx,q,1").is_err());
}

#[test]
fn parse_skips_blank_lines() {
    let csv = "h\n\nx,q,1,u,2,point,s,\"a, b\"\n";
    let t = parse_targets(csv).unwrap();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].scale, 2.0);
}

#[test]
fn clock_diff_wraps() {
    assert!((clock_diff(23.0, 1.0) + 2.0).abs() < 1e-12);
    assert!((clock_diff(1.0, 23.0) - 2.0).abs() < 1e-12);
    assert!(clock_diff(5.0, 5.0).abs() < 1e-12);
}

#[test]
fn clock_diff_range() {
    for i in 0..48 {
        for j in 0..48 {
            let d = clock_diff(i as f64 * 0.5, j as f64 * 0.5);
            assert!(d > -12.0 - 1e-9 && d <= 12.0 + 1e-9);
        }
    }
}

#[test]
fn upper_bound_residual_zero_below() {
    let t = Target {
        id: "a".into(),
        value: 60.0,
        scale: 60.0,
        kind: "upper_bound".into(),
        source: "s".into(),
    };
    assert_eq!(residual(&t, 30.0), 0.0);
    assert!((residual(&t, 90.0) - 0.5).abs() < 1e-12);
}

#[test]
fn point_residual_is_relative() {
    let t = Target {
        id: "a".into(),
        value: 2.0,
        scale: 2.0,
        kind: "point".into(),
        source: "s".into(),
    };
    assert!((residual(&t, 3.0) - 0.5).abs() < 1e-12);
}

#[test]
fn rmse_basic() {
    assert_eq!(rmse(&[]), 0.0);
    assert!((rmse(&[3.0, 4.0]) - (12.5f64).sqrt()).abs() < 1e-12);
}

#[test]
fn nelder_mead_finds_quadratic_minimum() {
    let (x, v) = nelder_mead(
        |z| (z[0] - 1.0).powi(2) + (z[1] + 2.0).powi(2),
        &[0.0, 0.0],
        0.5,
        500,
    );
    assert!(v < 1e-8 && (x[0] - 1.0).abs() < 1e-3 && (x[1] + 2.0).abs() < 1e-3);
}

#[test]
fn nelder_mead_rosenbrock() {
    let f = |z: &[f64]| (1.0 - z[0]).powi(2) + 100.0 * (z[1] - z[0] * z[0]).powi(2);
    let (x, _) = nelder_mead(f, &[-1.0, 1.0], 0.5, 5000);
    assert!(
        (x[0] - 1.0).abs() < 1e-2 && (x[1] - 1.0).abs() < 2e-2,
        "{x:?}"
    );
}

#[test]
fn apply_zero_is_identity_on_fitted_keys() {
    let b = Params::prior();
    let p = apply(&b, &[0.0; 9]);
    assert_eq!(p, b);
}

#[test]
fn apply_respects_bounds() {
    let p = apply(&Params::prior(), &[10.0; 9]);
    assert!(
        p.lep_a1 <= 0.8 && p.lep_a2 <= 0.4 && p.sleep_lep <= 0.9 && p.ghr_tau_s <= 4.0 * 3600.0
    );
    let q = apply(&Params::prior(), &[-10.0; 9]);
    assert!(q.lep_mean > 0.0 && q.ghr_tau_s >= 60.0);
}

#[test]
fn fitted_defaults_match_targets() {
    // Params::default() holds the fitted values: in-sample relative RMSE must stay small.
    let r: Vec<f64> = residuals(&Params::default(), &targets())
        .iter()
        .map(|x| x.2)
        .collect();
    assert!(rmse(&r) < 0.03, "rmse {}", rmse(&r));
}

#[test]
fn prior_fits_worse_than_default() {
    let ts = targets();
    let e = |p: &Params| rmse(&residuals(p, &ts).iter().map(|x| x.2).collect::<Vec<_>>());
    assert!(e(&Params::prior()) > e(&Params::default()));
}

#[test]
fn fit_improves_on_prior() {
    let ts = targets();
    let f = fit(&Params::prior(), &ts, 150);
    let r0 = rmse(
        &residuals(&Params::prior(), &ts)
            .iter()
            .map(|x| x.2)
            .collect::<Vec<_>>(),
    );
    assert!(f.rmse < r0);
}

#[test]
fn leptin_amplitude_close_to_sinha() {
    let s = summaries(&Params::default());
    let amp = s.iter().find(|x| x.0 == "lep_amp").unwrap().1;
    assert!((amp - 75.6).abs() < 3.0, "{amp}");
}

#[test]
fn glp1_peak_time_close_to_elliott() {
    let s = summaries(&Params::default());
    let t = s.iter().find(|x| x.0 == "glp1_tpeak").unwrap().1;
    assert!((t - 30.0).abs() <= 3.0, "{t}");
}

#[test]
fn ghrelin_ratio_close_to_cummings() {
    let s = summaries(&Params::default());
    let r = s.iter().find(|x| x.0 == "ghr_ratio").unwrap().1;
    assert!((r - 2.0).abs() < 0.1, "{r}");
}

#[test]
fn single_meal_day_has_one_meal() {
    let d = single_meal_day(75.0);
    assert_eq!(d.meals.len(), 1);
    assert_eq!(d.meals[0].carbs_g, 75.0);
}

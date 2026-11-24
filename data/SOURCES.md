# Data sources

All values in `targets.csv` come from published abstracts. Each abstract was read on 2026-11-21 through NCBI E-utilities (`efetch`, PubMed). The quote column holds the exact abstract text.

These values are **summary statistics** (ratios, peak times, means). They are not raw time series. No figure was digitized. The fit is therefore a fit to 9 published summary numbers, not to full curves.

| Key | Citation | PMID | DOI |
|---|---|---|---|
| cummings2001 | Cummings DE, Purnell JQ, Frayo RS, Schmidova K, Wisse BE, Weigle DS. A preprandial rise in plasma ghrelin levels suggests a role in meal initiation in humans. *Diabetes* 2001;50(8):1714-1719. | 11473029 | 10.2337/diabetes.50.8.1714 |
| sinha1996 | Sinha MK, Ohannesian JP, Heiman ML, et al. Nocturnal rise of leptin in lean, obese, and non-insulin-dependent diabetes mellitus subjects. *J Clin Invest* 1996;97(5):1344-1347. | 8636448 | 10.1172/JCI118551 |
| elliott1993 | Elliott RM, Morgan LM, Tredger JA, Deacon S, Wright J, Marks V. Glucagon-like peptide-1 (7-36)amide and glucose-dependent insulinotropic polypeptide secretion in response to nutrient ingestion in man. *J Endocrinol* 1993;138(1):159-166. | 7852887 | 10.1677/joe.0.1380159 |
| spiegel2004 | Spiegel K, Tasali E, Penev P, Van Cauter E. Sleep curtailment in healthy young men is associated with decreased leptin levels, elevated ghrelin levels, and increased hunger and appetite. *Ann Intern Med* 2004;141(11):846-850. | 15583226 | 10.7326/0003-4819-141-11-200412070-00008 |

## Interpretation choices (read before you trust the fit)

- **Leptin amplitude.** Sinha 1996 does not define "amplitude" in the abstract. The model uses (max - min) / min x 100. Another definition gives another number.
- **Ghrelin trough.** "Within 1 h" is an upper bound. The residual is 0 when the model trough comes at 60 min or earlier.
- **Meal times.** The abstracts do not give the meal clock times. The model uses 08:00, 12:30 and 18:30 (assumption).
- **Sleep loss.** Spiegel 2004 compared 4 h vs 10 h in bed for 2 nights. The model maps `sleep_h = 4` to that contrast and scales linearly between 4 h and 10 h (assumption).
- **Clock residuals** are circular differences in hours divided by 24 h.
- **Not used.** The insulin-ghrelin reciprocity (Cummings 2001) is a qualitative statement. It is a test, not a fit target. No AgRP or POMC firing data in humans was found, so the neural plant stays unfitted.

## v1.0 model terms and their sources (2015 and later)

The v0.2 fit targets above come from 1993-2004 abstracts. The v1.0 terms below use papers from 2015 on. Each paper was found and its abstract read on 2026-11-24 through NCBI E-utilities (`esearch`, `esummary`, `efetch`). Only numbers that appear in the abstract are used as literature values. Where a source gives no number, the test checks direction only. Every model parameter that no source gives is an **assumption** and is marked so.

| Key | Citation | PMID | DOI |
|---|---|---|---|
| mueller2015 | Müller TD, et al. Ghrelin. *Mol Metab* 2015;4(6):437-460. | 26042199 | 10.1016/j.molmet.2015.03.005 |
| mandelblat2015 | Mandelblat-Cerf Y, Ramesh RN, Burgess CR, Patella P, Yang Z, Lowell BB, Andermann ML. Arcuate hypothalamic AgRP and putative POMC neurons show opposite changes in spiking across multiple timescales. *eLife* 2015;4:e07122. | 26159614 | 10.7554/eLife.07122 |
| morris2015 | Morris CJ, Yang JN, Garcia JI, et al. Endogenous circadian system and circadian misalignment impact glucose tolerance via separate mechanisms in humans. *PNAS* 2015;112(17):E2225-E2234. | 25870289 | 10.1073/pnas.1418955112 |
| dashti2015 | Dashti HS, Scheer FA, Jacques PF, Lamon-Fava S, Ordovás JM. Short sleep duration and dietary intake: epidemiologic evidence, mechanisms, and health implications. *Adv Nutr* 2015;6(6):648-659. | 26567190 | 10.3945/an.115.008623 |
| stalder2016 | Stalder T, Kirschbaum C, Kudielka BM, et al. Assessment of the cortisol awakening response: Expert consensus guidelines. *Psychoneuroendocrinology* 2016;63:414-432. | 26563991 | 10.1016/j.psyneuen.2015.10.010 |
| qian2016 | Qian J, Scheer FAJL. Circadian system and glucose metabolism: implications for physiology and disease. *Trends Endocrinol Metab* 2016;27(5):282-293. | 27079518 | 10.1016/j.tem.2016.03.005 |
| svane2016 | Svane MS, Jørgensen NB, Bojsen-Møller KN, et al. Peptide YY and glucagon-like peptide-1 contribute to decreased food intake after Roux-en-Y gastric bypass surgery. *Int J Obes* 2016;40(11):1699-1706. | 27434221 | 10.1038/ijo.2016.121 |
| hjerpsted2018 | Hjerpsted JB, Flint A, Brooks A, Axelsen MB, Kvist T, Blundell J. Semaglutide improves postprandial glucose and lipid metabolism, and delays first-hour gastric emptying in subjects with obesity. *Diabetes Obes Metab* 2018;20(3):610-619. | 28941314 | 10.1111/dom.13120 |
| visentin2018 | Visentin R, et al. The UVA/Padova Type 1 Diabetes Simulator goes from single meal to single day. *J Diabetes Sci Technol* 2018;12(2):273-281. | 29451021 | 10.1177/1932296818757747 |

| Term | Parameters | Source and what is taken from it | Test (`rust/tests/terms.rs`) |
|---|---|---|---|
| Ghrelin | `ghr_base`, `ghr_rise`, `ghr_supp`, `ghr_tau_s`, `ghr_circ` | Mechanism: mueller2015 (review, no number used). Number: preprandial rise, Cummings 2001 (fit target above). | `lit_ghrelin_preprandial_rise_about_twofold`, `lit_ghrelin_trough_within_one_hour` |
| Leptin | `lep_*`, `sleep_lep` | Circadian shape and sleep effect: Cummings 2001, Sinha 1996, Spiegel 2004 (fit targets above). Circadian context: qian2016 (no number used). | `lit_leptin_zenith_near_01h_and_nadir_near_09h`, `lit_leptin_sleep_restriction_minus_18_percent` |
| Insulin | `ins_*` | Structure: minimal model with an incretin term (assumption). Context: visentin2018 (qualitative: intraday insulin sensitivity varies). **Known mismatch:** hjerpsted2018 reports a lower postprandial insulin area with semaglutide. The model gives a higher one. | `lit_insulin_agonist_mismatch_is_known` |
| GLP-1 | `glp1_*` | Peak 30 min after carbohydrate: Elliott 1993 (fit target above). | `lit_glp1_peak_30_min_after_carbohydrate` |
| PYY | `pyy_base`, `pyy_gain`, `pyy_tau_s`, `w_pyy`, `p_pyy` | svane2016 (PYY3-36 and GLP-1 both reduce intake; no baseline number in the abstract). hjerpsted2018: postprandial PYY response lower with semaglutide (direction used). **Scale, time constant and coupling weights are assumptions.** | `lit_pyy_postprandial_response_lower_with_agonist`, `pyy_*` |
| Cortisol | `cort_*`, `car_*`, `w_cort` | stalder2016: the awakening response is the rise over the first 30-45 min after waking (timing used). Circadian amplitude, stress gain and sleep-debt gain are **assumptions**. | `lit_cortisol_awakening_peak_30_to_45_min`, `lit_cortisol_morning_high_midnight_low` |
| Gastric emptying | `gastric_k`, `drug_empt_slow` | hjerpsted2018: first-hour emptying ratio 0.73 (95% CI 0.61 to 0.87) with semaglutide. `drug_empt_slow = 1.05` is set so the model gives 0.730. The 40-min half-time of `gastric_k` is an assumption. | `lit_gastric_first_hour_emptying_ratio_inside_ci` |
| Glucose | `glu_*`, `misalign_isens` | morris2015: 12-h circadian misalignment raised postprandial glucose by 6%. `misalign_isens = 0.6` is set to give about half of that (1.031). The rest of the effect has other causes that the model does not have. hjerpsted2018: glucose area lower with semaglutide (direction used). | `lit_misalignment_raises_postprandial_glucose`, `lit_glucose_postprandial_area_lower_with_agonist` |
| Sleep debt | `sleep_debt()`, `debt_tau_s`, `cort_debt` | Acute severity (4 h vs 10 h in bed): Spiegel 2004. Context: dashti2015 (short sleep raises intake; no number used). The 36-h time constant of the cumulative debt is an **assumption**. | `lit_sleep_debt_follows_36h_time_constant`, `lit_ghrelin_sleep_restriction_plus_28_percent` |
| AgRP and POMC coupling | `Coupling` weights | mandelblat2015 (mouse): AgRP firing rose about fivefold with caloric deficit; putative POMC neurons moved the other way. Direction used as a test. Weights are assumptions. | `lit_agrp_drive_afternoon_exceeds_late_morning`, `lit_pomc_moves_opposite_to_agrp` |
| Illness | `ill_ghr` | **Assumption.** No source number used. Illness lowers the ghrelin target and raises cortisol in the model. | `illness_lowers_ghrelin_target` |

Mouse data (mandelblat2015) tell us about direction only. No term in this model is fitted to human neural data.

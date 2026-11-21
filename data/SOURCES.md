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

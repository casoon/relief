//! Risikoklasse unter Hypothesen (→ `plan/spezifikation/05`): Eine KI darf
//! die Einstufung nur erhöhen, nie senken.

use crate::Hypothesis;

pub use relief_interaction::Risk;

/// Risikoklasse einer Aktion, wenn Hypothesen vorliegen.
///
/// `assess(None)` stuft allein nach dem ein, was ohne Modell bekannt ist;
/// `assess(Some(h))` zusätzlich mit der Hypothese `h` (z. B. einem
/// erschlossenen Namen). Ergebnis ist das Maximum: Eine Hypothese kann eine
/// Rückfrage auslösen, aber nie eine ersparen.
pub fn assess_risk<F>(assess: F, hypotheses: &[Hypothesis]) -> Risk
where
    F: Fn(Option<&Hypothesis>) -> Risk,
{
    hypotheses
        .iter()
        .map(|h| assess(Some(h)))
        .fold(assess(None), Risk::max)
}

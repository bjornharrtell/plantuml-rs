//! Database and queue symbols in their small form, see [`crate::decoration::usymbol`].

use crate::decoration::usymbol::usymbols;
use crate::klimt::fashion::Fashion;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use std::rc::Rc;

/// Small database symbol around `label` (`USymbolDatabase.asSmall`).
pub fn database_small(
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
) -> Rc<dyn TextBlock> {
    usymbols::database().as_small(
        None,
        label,
        stereotype,
        fashion,
        HorizontalAlignment::Center,
    )
}

/// Small queue symbol around `label` (`USymbolQueue.asSmall`).
pub fn queue_small(
    label: Rc<dyn TextBlock>,
    stereotype: Rc<dyn TextBlock>,
    fashion: Fashion,
) -> Rc<dyn TextBlock> {
    usymbols::queue().as_small(
        None,
        label,
        stereotype,
        fashion,
        HorizontalAlignment::Center,
    )
}

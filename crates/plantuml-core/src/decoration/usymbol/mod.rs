//! Shapes of elements and packages (`USymbol` and `USymbols`).

use crate::klimt::fashion::Fashion;
use crate::klimt::geom::HorizontalAlignment;
use crate::klimt::shape::TextBlock;
use crate::style::SName;
use std::rc::Rc;

/// A shape that can hold a label (`USymbol`).
pub trait USymbol {
    /// Style names added to the signature of the element (`getSNames`).
    fn snames(&self) -> Vec<SName>;

    /// The symbol around a label (`asSmall`).
    fn as_small(
        &self,
        name: Option<Rc<dyn TextBlock>>,
        label: Rc<dyn TextBlock>,
        stereotype: Rc<dyn TextBlock>,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock>;

    /// The symbol of a given size, used for packages (`asBig`).
    #[allow(clippy::too_many_arguments)]
    fn as_big(
        &self,
        label: Rc<dyn TextBlock>,
        label_alignment: HorizontalAlignment,
        stereotype: Rc<dyn TextBlock>,
        width: f64,
        height: f64,
        context: Fashion,
        stereo_alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock>;

    fn supp_height_because_of_shape(&self) -> i32 {
        0
    }

    fn supp_width_because_of_shape(&self) -> i32 {
        0
    }
}

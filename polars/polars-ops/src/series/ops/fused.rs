use arrow::array::PrimitiveArray;
use polars_arrow::prelude::ArrayRef;
use polars_arrow::utils::combine_validities_and;
use polars_core::prelude::*;
use polars_core::utils::align_chunks_ternary;
use polars_core::with_match_physical_numeric_polars_type;

// a + (b * c)
fn fma_arr<T: NumericNative>(
    a: &PrimitiveArray<T>,
    b: &PrimitiveArray<T>,
    c: &PrimitiveArray<T>,
) -> PrimitiveArray<T> {
    assert_eq!(a.len(), b.len());
    let validity = combine_validities_and(
        combine_validities_and(a.validity(), b.validity()).as_ref(),
        c.validity(),
    );
    let a = a.values().as_slice();
    let b = b.values().as_slice();
    let c = c.values().as_slice();

    assert_eq!(a.len(), b.len());
    assert_eq!(b.len(), c.len());
    let out = a
        .iter()
        .zip(b.iter())
        .zip(c.iter())
        .map(|((a, b), c)| *a + (*b * *c))
        .collect::<Vec<_>>();
    PrimitiveArray::from_data_default(out.into(), validity)
}

fn fma_ca<T: PolarsNumericType>(
    a: &ChunkedArray<T>,
    b: &ChunkedArray<T>,
    c: &ChunkedArray<T>,
) -> ChunkedArray<T> {
    let (a, b, c) = align_chunks_ternary(a, b, c);

    let chunks = a
        .downcast_iter()
        .zip(b.downcast_iter())
        .zip(c.downcast_iter())
        .map(|((a, b), c)| Box::new(fma_arr(a, b, c)) as ArrayRef)
        .collect();

    // safety: same dtypes
    unsafe { ChunkedArray::from_chunks(a.name(), chunks) }
}

pub fn fma_series(a: &Series, b: &Series, c: &Series) -> Series {
    if a.len() == b.len() && a.len() == c.len() {
        with_match_physical_numeric_polars_type!(a.dtype(), |$T| {
            let a: &ChunkedArray<$T> = a.as_ref().as_ref().as_ref();
            let b: &ChunkedArray<$T> = b.as_ref().as_ref().as_ref();
            let c: &ChunkedArray<$T> = c.as_ref().as_ref().as_ref();

            fma_ca(a, b, c).into_series()
        })
    } else {
        a + &(b * c)
    }
}

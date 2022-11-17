/// Either a single T or a Vec<T> of values.
///
/// Reduces heap allocations.
#[derive(Debug)]
pub(crate) enum MaybeScalar<T> {
	Scalar(T),
	Vec(Vec<T>),
}
impl<T> MaybeScalar<T> {
	pub fn as_vec(&mut self) -> &mut Vec<T> {
		*self = match core::mem::take(self) {
			Self::Scalar(scalar) => Self::Vec(vec![scalar]),
			unchanged @ Self::Vec(_) => unchanged,
		};
		match self {
			Self::Vec(vec) => vec,
			_ => unreachable!(),
		}
	}
}
impl<T> Default for MaybeScalar<T> {
	fn default() -> Self {
		Self::Vec(Vec::new())
	}
}
impl<T> From<T> for MaybeScalar<T> {
	#[inline(always)]
	fn from(t: T) -> Self {
		MaybeScalar::Scalar(t)
	}
}
impl<T> From<Vec<T>> for MaybeScalar<T> {
	#[inline(always)]
	fn from(t: Vec<T>) -> Self {
		MaybeScalar::Vec(t)
	}
}
impl<T> AsRef<[T]> for MaybeScalar<T> {
	#[inline(always)]
	fn as_ref(&self) -> &[T] {
		match self {
			Self::Scalar(t) => core::slice::from_ref(t),
			Self::Vec(t) => t.as_ref(),
		}
	}
}
impl<T> AsMut<[T]> for MaybeScalar<T> {
	#[inline(always)]
	fn as_mut(&mut self) -> &mut [T] {
		match self {
			Self::Scalar(t) => core::slice::from_mut(t),
			Self::Vec(t) => t.as_mut(),
		}
	}
}

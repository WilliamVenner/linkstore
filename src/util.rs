pub(crate) enum MaybeOwnedBytes<'a> {
	Borrowed(&'a [u8]),
	Owned(Vec<u8>)
}
impl AsRef<[u8]> for MaybeOwnedBytes<'_> {
	#[inline(always)]
    fn as_ref(&self) -> &[u8] {
        match self {
			Self::Borrowed(bytes) => bytes,
			Self::Owned(owned) => owned
		}
    }
}

pub(crate) enum MaybeScalar<T> {
	Scalar(T),
	Vec(Vec<T>)
}
impl<T> MaybeScalar<T> {
	pub fn as_vec(&mut self) -> &mut Vec<T> {
		*self = match core::mem::take(self) {
			Self::Scalar(scalar) => Self::Vec(vec![scalar]),
			unchanged @ Self::Vec(_) => unchanged
		};
		match self {
			Self::Vec(vec) => vec,
			_ => unreachable!()
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
			Self::Vec(t) => t.as_mut()
		}
	}
}
impl<T> IntoIterator for MaybeScalar<T> {
	type Item = T;
	type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

	fn into_iter(self) -> Self::IntoIter {
		match self {
			Self::Scalar(t) => vec![t].into_iter(),
			Self::Vec(t) => t.into_iter(),
		}
	}
}
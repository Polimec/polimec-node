use did::did_details::DidDetails;
use pallet_dip_provider::traits::IdentityProvider;
use sp_std::marker::PhantomData;

pub struct DidIdentityProvider<T>(PhantomData<T>);

impl<T> IdentityProvider<T::DidIdentifier, DidDetails<T>, ()> for DidIdentityProvider<T>
where
	T: did::Config,
{
	// TODO: Proper error handling
	type Error = ();

	fn retrieve(identifier: &T::DidIdentifier) -> Result<Option<(DidDetails<T>, ())>, Self::Error> {
		match (
			did::Pallet::<T>::get_did(identifier),
			did::Pallet::<T>::get_deleted_did(identifier),
		) {
			(Some(details), _) => Ok(Some((details, ()))),
			(_, Some(_)) => Ok(None),
			_ => Err(()),
		}
	}
}
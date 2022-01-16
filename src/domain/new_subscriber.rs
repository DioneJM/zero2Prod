use crate::domain::SubscriberName;
use crate::domain::subscriber_email::SubscriberEmail;
use std::prelude::rust_2021::{TryFrom, TryInto};
use crate::FormData;

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<NewSubscriber, String> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(NewSubscriber { name, email })
    }
}

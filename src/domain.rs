
pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName
}
pub struct SubscriberName(String);

impl SubscriberName {

    pub fn parse(s: String) -> SubscriberName {
        let is_empty_or_whitespace = s.trim().is_empty();

        let is_too_long = s.chars().count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars()
            .any(|c| forbidden_characters.contains(&c));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            panic!("{} isn ot a valid subscriber name", s)
        } else {
            Self(s)
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
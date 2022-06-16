use alloc::boxed::Box;

pub struct Redirect(Box<str>);

impl Redirect {
    pub fn new(id: &str, redirect: &str) -> Self {
        let form = alloc::format!(
            "https://discord.com/api/oauth2/authorize?response_type=code&scope=identify&client_id={id}&redirect_uri={redirect}&state="
        );
        Self(form.into_boxed_str())
    }

    pub fn generate_consent_page_uri(&self, state: &str) -> Box<str> {
        let uri = self.0.clone().into_string() + state;
        uri.into_boxed_str()
    }
}

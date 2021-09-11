use serde::{
    de::{Error, Unexpected},
    Deserialize, Deserializer,
};
use serde_json::{Map, Value};

pub struct Interaction {
    pub interaction_id: u64,
    pub application_id: u64,
    pub user_id: u64,
    pub data: InteractionData,
    pub token: Box<str>,
}

pub enum InteractionData {
    Ping,
    AppCommand {
        command_id: u64,
        name: Box<str>,
        url: Box<str>,
    },
    SelectMenu {
        custom_id: Box<str>,
        selection: Box<str>,
    },
}

impl<'de> Deserialize<'de> for Interaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Parse metadata at the interaction-level
        let mut map = Map::deserialize(deserializer)?;
        let interaction_id = map
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(D::Error::missing_field("interaction id"))?;
        let application_id = map
            .get("application_id")
            .and_then(Value::as_u64)
            .ok_or(D::Error::missing_field("application id"))?;

        // Parse interaction token
        let maybe_token = map
            .remove("token")
            .ok_or(D::Error::missing_field("interaction token"))?;
        let token = match maybe_token {
            Value::String(text) => text.into_boxed_str(),
            _ => {
                return Err(D::Error::invalid_type(
                    Unexpected::Other("non-string token"),
                    &"string token",
                ))
            }
        };

        // Parse the user ID
        let user_id = map
            .get("member")
            .and_then(|member| member.as_object()?.get("user"))
            .xor(map.get("user"))
            .and_then(|user| user.as_object()?.get("id")?.as_u64())
            .ok_or(D::Error::missing_field("user id"))?;

        // Resolve data union
        let interaction_type = map
            .get("type")
            .and_then(Value::as_u64)
            .ok_or(D::Error::missing_field("interaction type"))?;
        let data = match interaction_type {
            1 => InteractionData::Ping,
            2 => {
                let data = map
                    .get_mut("data")
                    .and_then(Value::as_object_mut)
                    .ok_or(D::Error::missing_field("data"))?;
                let command_id = data
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or(D::Error::missing_field("command id"))?;
                let maybe_name = data
                    .remove("name")
                    .ok_or(D::Error::missing_field("command name"))?;
                let name = match maybe_name {
                    Value::String(text) => text.into_boxed_str(),
                    _ => {
                        return Err(D::Error::invalid_type(
                            Unexpected::Other("non-string token"),
                            &"string token",
                        ))
                    }
                };

                let argument = data
                    .get_mut("options")
                    .and_then(|val| val.as_array_mut()?.first_mut()?.as_object_mut())
                    .ok_or(D::Error::missing_field("command argument for url"))?;
                let arg_name = argument
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if arg_name != "url" {
                    return Err(D::Error::invalid_value(Unexpected::Str(arg_name), &"url"));
                }

                let maybe_url = argument
                    .remove("value")
                    .ok_or(D::Error::missing_field("argument value"))?;
                let url = match maybe_url {
                    Value::String(text) => text.into_boxed_str(),
                    _ => {
                        return Err(D::Error::invalid_type(
                            Unexpected::Other("non-string token"),
                            &"string token",
                        ))
                    }
                };

                InteractionData::AppCommand {
                    command_id,
                    name,
                    url,
                }
            }
            3 => {
                let data = map
                    .get_mut("data")
                    .and_then(Value::as_object_mut)
                    .ok_or(D::Error::missing_field("data"))?;
                let values = data
                    .get_mut("values")
                    .and_then(Value::as_array_mut)
                    .ok_or(D::Error::missing_field("url parameter"))?;
                if values.is_empty() {
                    return Err(D::Error::invalid_length(0, &"non-empty values"));
                }

                let selection = match values.swap_remove(0) {
                    Value::String(text) => text.into_boxed_str(),
                    _ => {
                        return Err(D::Error::invalid_type(
                            Unexpected::Other("non-string selection"),
                            &"string selection",
                        ))
                    }
                };
                let maybe_custom_id = data
                    .remove("id")
                    .ok_or(D::Error::missing_field("custom id"))?;
                let custom_id = match maybe_custom_id {
                    Value::String(text) => text.into_boxed_str(),
                    _ => {
                        return Err(D::Error::invalid_type(
                            Unexpected::Other("non-string selection"),
                            &"string selection",
                        ))
                    }
                };

                InteractionData::SelectMenu {
                    custom_id,
                    selection,
                }
            }
            _ => {
                return Err(D::Error::invalid_value(
                    Unexpected::Other("unsupported interaction type"),
                    &"valid interaction type",
                ))
            }
        };

        Ok(Self {
            interaction_id,
            application_id,
            user_id,
            token,
            data,
        })
    }
}

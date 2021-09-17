use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{collections::HashMap, num::NonZeroU64};

pub struct Interaction<'txt> {
    pub interaction_id: NonZeroU64,
    pub application_id: NonZeroU64,
    pub user_id: NonZeroU64,
    pub data: InteractionData<'txt>,
    pub token: &'txt str,
}

pub enum InteractionData<'txt> {
    Ping,
    AppCommand {
        command_id: NonZeroU64,
        name: &'txt str,
        url: &'txt str,
    },
    SelectMenu {
        custom_id: &'txt str,
        selection: &'txt str,
    },
}

pub enum InteractionResponse<'a> {
    Pong,
    ChannelMessageWithSource(InteractionCallbackData<'a>),
    DeferredChannelMessageWithSource(InteractionCallbackData<'a>),
    DeferredUpdateMessage(InteractionCallbackData<'a>),
    UpdateMessage(InteractionCallbackData<'a>),
}

impl InteractionResponse<'_> {
    fn try_as_data(&self) -> Option<&InteractionCallbackData> {
        match self {
            Self::ChannelMessageWithSource(data)
            | Self::DeferredChannelMessageWithSource(data)
            | Self::DeferredUpdateMessage(data)
            | Self::UpdateMessage(data) => Some(data),
            _ => None,
        }
    }

    fn to_int(&self) -> u8 {
        match *self {
            Self::Pong => 1,
            Self::ChannelMessageWithSource(_) => 4,
            Self::DeferredChannelMessageWithSource(_) => 5,
            Self::DeferredUpdateMessage(_) => 6,
            Self::UpdateMessage(_) => 7,
        }
    }
}

impl Serialize for InteractionResponse<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let field_count = match self {
            Self::Pong => 1,
            _ => 2,
        };

        let mut s = serializer.serialize_struct("InteractionResponse", field_count)?;
        s.serialize_field("type", &self.to_int())?;
        let data = match self.try_as_data() {
            Some(d) => d,
            _ => return s.end(),
        };
        s.serialize_field("data", data)?;
        s.end()
    }
}

fn skip_if_false(ephemeral: &bool) -> bool {
    !ephemeral
}

fn as_ephemeral_int<S>(ephemeral: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u8(if *ephemeral { 1 << 6 } else { 0 })
}

fn as_allowed_mentions<S>(allow_user_mentions: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut structure = serializer.serialize_struct("AllowedMentions", 1)?;
    let parse = if *allow_user_mentions { &["users"][..] } else { &[][..] };
    structure.serialize_field("parse", parse)?;
    structure.end()
}

fn slice_is_empty<T>(items: &[T]) -> bool {
    items.is_empty()
}

#[derive(Serialize)]
pub struct InteractionCallbackData<'txt> {
    pub content: &'txt str,
    #[serde(skip_serializing_if = "skip_if_false")]
    #[serde(rename = "flags")]
    #[serde(serialize_with = "as_ephemeral_int")]
    pub ephemeral: bool,
    #[serde(skip_serializing_if = "skip_if_false")]
    #[serde(rename = "allowed_mentions")]
    #[serde(serialize_with = "as_allowed_mentions")]
    pub allow_user_mentions: bool,
    #[serde(skip_serializing_if = "slice_is_empty")]
    pub components: &'txt [ActionRow<'txt>],
}

impl InteractionCallbackData<'static> {
    pub const FOUND_QUIZ: Self = Self {
        content: "Your response has been recorded.",
        ephemeral: true,
        allow_user_mentions: false,
        components: &[],
    };

    pub const MISSING_QUIZ: Self = Self {
        content: "Cannot find quiz.",
        ephemeral: true,
        allow_user_mentions: false,
        components: &[],
    };

    pub const EXPIRED_QUIZ: Self = Self {
        content: "Quiz has already expired.",
        ephemeral: true,
        allow_user_mentions: false,
        components: &[],
    };

    pub const FETCH_ERROR: Self = Self {
        content: "Cannot fetch proper JSON schema.",
        ephemeral: true,
        allow_user_mentions: false,
        components: &[],
    };

    pub const MALFORMED_QUIZ: Self = Self {
        content: "Cannot proceed because the quiz is malformed.",
        ephemeral: true,
        allow_user_mentions: false,
        components: &[],
    };

    pub const MALFORMED_URL: Self = Self {
        content: "Malformed URL given.",
        ephemeral: true,
        allow_user_mentions: false,
        components: &[],
    };
}

#[derive(Serialize)]
pub struct SelectMenuOption<'txt> {
    /// User-facing text.
    label: &'txt str,
    /// Dev-defined text.
    value: &'txt str,
    /// User-facing description accompanying the `label`.
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'txt str>,
}

pub enum MessageComponentVariant<'a> {
    Button {
        /// User-facing value of the button.
        value: &'a str,
    },
    SelectMenu {
        /// Temporary value for the menu select.
        placeholder: &'a str,
        /// Values for each item in the dropdown.
        options: &'a [SelectMenuOption<'a>],
    },
}

impl Serialize for MessageComponentVariant<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (placeholder, options) = match *self {
            Self::SelectMenu { placeholder, options } => (placeholder, options),
            _ => todo!(),
        };
        let mut s = serializer.serialize_struct("MessageComponentVariant", 3)?;
        s.serialize_field("type", &3)?;
        s.serialize_field("placeholder", placeholder)?;
        s.serialize_field("options", options)?;
        s.end()
    }
}

#[derive(Serialize)]
pub struct MessageComponent<'txt> {
    pub custom_id: &'txt str,
    #[serde(flatten)]
    pub variant: MessageComponentVariant<'txt>,
}

pub struct ActionRow<'c>(&'c [MessageComponent<'c>]);

impl Serialize for ActionRow<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut obj = serializer.serialize_struct("MessageComponent", 2)?;
        obj.serialize_field("type", &1)?;
        obj.serialize_field("components", self.0)?;
        obj.end()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DiscordField<'txt> {
    Num(NonZeroU64),
    Str(&'txt str),
    Seq(Box<[Self]>),
    Map(HashMap<&'txt str, Self>),
}

impl<'a> DiscordField<'a> {
    fn into_snowflake(self) -> Option<NonZeroU64> {
        match self {
            Self::Num(id) => Some(id),
            _ => None,
        }
    }

    fn into_str(self) -> Option<&'a str> {
        match self {
            Self::Str(inner) => Some(inner),
            _ => None,
        }
    }

    fn into_seq(self) -> Option<Box<[Self]>> {
        match self {
            Self::Seq(inner) => Some(inner),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Interaction<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct InteractionVisitor;

        impl<'de> Visitor<'de> for InteractionVisitor {
            type Value = Interaction<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a valid value from Discord")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // Initialize expected fields
                let mut interaction_id = None::<NonZeroU64>;
                let mut application_id = None::<NonZeroU64>;
                let mut user_id = None::<NonZeroU64>;
                let mut token = None::<&str>;
                let mut data = None::<HashMap<&str, DiscordField>>;
                let mut data_type = None::<NonZeroU64>;

                // Check for correct key-value pairs
                while let Some(pair) = map.next_entry()? {
                    let mut user = match pair {
                        ("id", DiscordField::Num(id)) => {
                            interaction_id = Some(id);
                            continue;
                        }
                        ("application_id", DiscordField::Num(id)) => {
                            application_id = Some(id);
                            continue;
                        }
                        ("token", DiscordField::Str(tok)) => {
                            token = Some(tok);
                            continue;
                        }
                        ("type", DiscordField::Num(interaction_type)) => {
                            data_type = Some(interaction_type);
                            continue;
                        }
                        ("data", DiscordField::Map(interaction_data)) => {
                            data = Some(interaction_data);
                            continue;
                        }
                        ("user", DiscordField::Map(user)) => user,
                        ("member", DiscordField::Map(mut member)) => {
                            if let Some(DiscordField::Map(user)) = member.remove("user") {
                                user
                            } else {
                                continue;
                            }
                        }
                        _ => continue,
                    };
                    user_id = user.remove("id").and_then(DiscordField::into_snowflake);
                }

                // Deserialize interaction data
                const EXPECTED_INTERACTION_TYPES: [&str; 3] = ["PING", "APPLICATION_COMMAND", "MESSAGE_COMPONENT"];
                let interaction_type = data_type.ok_or(de::Error::missing_field("type"))?.get();
                let mut interaction_data = data.ok_or(de::Error::missing_field("data"))?;
                let data = match interaction_type {
                    1 => InteractionData::Ping,
                    2 => {
                        let command_type = match interaction_data.remove("type").and_then(DiscordField::into_snowflake)
                        {
                            Some(val) => val.get(),
                            _ => 0,
                        };
                        if command_type != 1 {
                            return Err(de::Error::unknown_variant("USER or MESSAGE", &["CHAT_INPUT"]));
                        }

                        let options = interaction_data
                            .remove("options")
                            .and_then(DiscordField::into_seq)
                            .ok_or(de::Error::missing_field("data.options"))?;
                        let url = match *options {
                            [DiscordField::Str(first), ..] => first,
                            _ => return Err(de::Error::invalid_length(0, &"non-empty")),
                        };
                        let name = interaction_data
                            .remove("name")
                            .and_then(DiscordField::into_str)
                            .ok_or(de::Error::missing_field("data.name"))?;
                        let command_id = interaction_data
                            .remove("id")
                            .and_then(DiscordField::into_snowflake)
                            .ok_or(de::Error::missing_field("data.id"))?;

                        InteractionData::AppCommand { command_id, name, url }
                    }
                    3 => {
                        let component_type = interaction_data
                            .remove("component_type")
                            .and_then(DiscordField::into_snowflake)
                            .ok_or(de::Error::missing_field("data.component_type"))?
                            .get();
                        if component_type != 3 {
                            return Err(de::Error::unknown_variant("ACTION_ROW or BUTTON", &["SELECT_MENU"]));
                        }

                        let values = interaction_data
                            .remove("values")
                            .and_then(DiscordField::into_seq)
                            .ok_or(de::Error::missing_field("data.values"))?;
                        let selection = match *values {
                            [DiscordField::Str(first), ..] => first,
                            _ => return Err(de::Error::invalid_length(0, &"non-empty")),
                        };
                        let custom_id = interaction_data
                            .remove("custom_id")
                            .and_then(DiscordField::into_str)
                            .ok_or(de::Error::missing_field("data.custom_id"))?;
                        InteractionData::SelectMenu { custom_id, selection }
                    }
                    _ => return Err(de::Error::unknown_variant("UNKNOWN", &EXPECTED_INTERACTION_TYPES)),
                };

                Ok(Interaction {
                    interaction_id: interaction_id.ok_or(de::Error::missing_field("id"))?,
                    application_id: application_id.ok_or(de::Error::missing_field("application_id"))?,
                    user_id: user_id.ok_or(de::Error::missing_field("user.id or member.user.id"))?,
                    token: token.ok_or(de::Error::missing_field("token"))?,
                    data,
                })
            }
        }

        const FIELDS: [&str; 6] = ["id", "application_id", "type", "data", "member", "user"];
        deserializer.deserialize_struct("Interaction", &FIELDS, InteractionVisitor)
    }
}

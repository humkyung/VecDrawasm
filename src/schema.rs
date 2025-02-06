diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        email -> Nullable<Text>,
    }
}
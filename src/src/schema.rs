table! {
    comments (id) {
        id -> Int4,
        slug -> Varchar,
        name -> Varchar,
        date -> Timestamptz,
        parent -> Nullable<Int4>,
        text -> Varchar,
        email -> Nullable<Varchar>,
    }
}

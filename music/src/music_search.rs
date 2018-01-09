pub enum Tag {
    Any(String),
    Title(String),
    Album(String),
    Interpret(String),
    Conductor(String),
    Composer(String)
}

impl Tag {
    pub fn from_search_query(query: &str) -> Option<Tag> {
        let elms = query.split(':').collect::<Vec<&str>>();

        if elms.len() == 1 {
            Some(Tag::Any(elms[0].into()))
        } else {
            match elms[0] {
                "title" | "TITLE" => Some(Tag::Title(elms[1].into())),
                "album" | "ALBUM" => Some(Tag::Album(elms[1].into())),
                "interpret" | "INTERPRET" => Some(Tag::Interpret(elms[1].into())),
                "conductor" | "performer" | "CONDUCTOR" | "PERFORMER" => Some(Tag::Conductor(elms[1].into())),
                "composer" | "COMPOSER" => Some(Tag::Composer(elms[1].into())),
                _ => return None
            }
        }
    }

    pub fn to_sql_query(self) -> String {
        match self {
            Tag::Any(x) => format!("Title LIKE '%{}%' OR Album LIKE '%{}%' OR Interpret LIKE '%{}%' OR Conductor LIKE '%{}' OR Composer LIKE '%{}%'", x, x, x, x, x),
            Tag::Title(x) => format!("Title LIKE '%{}%'", x),
            Tag::Album(x) => format!("Album LIKE '%{}%'", x),
            Tag::Interpret(x) => format!("Interpret LIKE '%{}%'", x),
            Tag::Conductor(x) => format!("Conductor LIKE '%{}%'", x),
            Tag::Composer(x) => format!("Composer LIKE '%{}%'", x)
        }
    }
}

pub struct SearchQuery {
    tags: Vec<Tag>
}

impl SearchQuery {
    pub fn new(input: &str) -> Option<SearchQuery> {
        let tags = input.split(',').filter_map(Tag::from_search_query).collect();

        Some(SearchQuery { tags: tags })
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    pub fn to_sql_query(self) -> String {
        self.tags.into_iter().map(|x| x.to_sql_query()).collect::<Vec<String>>().join(" OR ")
    }
}

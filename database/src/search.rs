//! Search query parser
//!
//! This module parses a query and converts it into a SQL statement. This statement can be used in
//! the database to search for tracks.

/// Enum providing allowed tags in the search query, like 'title:Crazy'
pub enum Tag {
    Any(String),
    Title(String),
    Album(String),
    Interpret(String),
    People(String),
    Composer(String)
}

/// Order by certain field
pub enum Order {
    ByID,
    ByTitle,
    ByFavs
}

impl Order {
    /// Create a new ordering from a query
    pub fn from_search_query(query: &str) -> Option<Order> {
        let elms = query.split(':').collect::<Vec<&str>>();

        if elms.len() == 2 && elms[0] == "order" {
            return match elms[1] {
                "id" => Some(Order::ByID),
                "title" => Some(Order::ByTitle),
                "favs" => Some(Order::ByFavs),
                _ => None
            };
        }

        None
    }

    /// Stringify the enum
    pub fn name(&self) -> String {
        let tmp = match *self {
            Order::ByID => "Title",
            Order::ByTitle => "Title",
            Order::ByFavs => "FavsCount"
        };

        tmp.into()
    }
}

impl Tag {
    /// Create a new tag from a search query
    pub fn from_search_query(query: &str) -> Option<Tag> {
        let elms = query.split(':').collect::<Vec<&str>>();

        if elms.len() == 1 {
            Some(Tag::Any(elms[0].into()))
        } else {
            match elms[0] {
                "title" | "TITLE" => Some(Tag::Title(elms[1].into())),
                "album" | "ALBUM" => Some(Tag::Album(elms[1].into())),
                "interpret" | "INTERPRET" => Some(Tag::Interpret(elms[1].into())),
                "people" | "performer" | "PEOPLE" | "PERFORMER" => Some(Tag::People(elms[1].into())),
                "composer" | "COMPOSER" => Some(Tag::Composer(elms[1].into())),
                _ => return None
            }
        }
    }

    /// Converts the tag to a SQL statement
    pub fn to_sql_query(self) -> String {
        match self {
            Tag::Any(x) => format!("Title LIKE '%{}%' OR Album LIKE '%{}%' OR Interpret LIKE '%{}%' OR People LIKE '%{}' OR Composer LIKE '%{}%'", x, x, x, x, x),
            Tag::Title(x) => format!("Title LIKE '%{}%'", x),
            Tag::Album(x) => format!("Album LIKE '%{}%'", x),
            Tag::Interpret(x) => format!("Interpret LIKE '%{}%'", x),
            Tag::People(x) => format!("People LIKE '%{}%'", x),
            Tag::Composer(x) => format!("Composer LIKE '%{}%'", x)
        }
    }
}

/// A search query consists of serveral tags and an ordering
pub struct SearchQuery {
    tags: Vec<Tag>,
    order: Order
}

impl SearchQuery {
    /// Create a new search query
    pub fn new(input: &str) -> Option<SearchQuery> {
        let tags = input.split(',').filter_map(Tag::from_search_query).collect();
        let order = input.split(',').filter_map(Order::from_search_query).next().unwrap_or(Order::ByID);

        Some(SearchQuery { tags: tags, order: order })
    }

    /// Check for emptiness
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Converts the search query to SQL
    pub fn to_sql_query(self) -> String {
        let mut tmp: String = "SELECT Title, Album, Interpret, Fingerprint, People, Composer, Key, Duration, FavsCount, Channels FROM music".into();
        
        if !self.tags.is_empty() {
            tmp.push_str(" WHERE ");
            tmp.push_str(&self.tags.into_iter().map(|x| x.to_sql_query()).collect::<Vec<String>>().join(" AND "));
        }

        match self.order {
            Order::ByTitle | Order::ByFavs => {
                tmp.push_str(" ORDER BY ");
                tmp.push_str(&self.order.name());
                tmp.push_str(" DESC");
            }
            _ => {}
        }

        tmp
    }
}

//! Search query parser
//!
//! This module parses a query and converts it into a SQL statement. This statement can be used in
//! the database to search for tracks.

/// Enum providing allowed tags in the search query, like 'title:Crazy'
#[derive(Debug)]
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
    ByDate,
    ByTitle,
    ByFavs
}

impl Order {
    /// Create a new ordering from a query
    pub fn from_search_query(query: &str) -> Option<Order> {
        let elms = query.split(':').collect::<Vec<&str>>();

        if elms.len() == 2 && elms[0] == "order" {
            return match elms[1] {
                "date" => Some(Order::ByDate),
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
            Order::ByDate => "Created",
            Order::ByTitle => "Title",
            Order::ByFavs => "FavsCount"
        };

        tmp.into()
    }
}

impl Tag {
    /// Create a new tag from a search query
    pub fn from_search_query(query: &str) -> Option<Tag> {
        let elms = query.split(':').map(|x| x.into()).collect::<Vec<String>>();

        if elms.len() == 1 {
            Some(Tag::Any(elms[0].replace("'", "''")))
        } else {
            let content = elms[1].replace("'", "''");

            match elms[0].as_str() {
                "title" | "TITLE" => Some(Tag::Title(content)),
                "album" | "ALBUM" => Some(Tag::Album(content)),
                "interpret" | "INTERPRET" => Some(Tag::Interpret(content)),
                "people" | "performer" | "PEOPLE" | "PERFORMER" => Some(Tag::People(content)),
                "composer" | "COMPOSER" => Some(Tag::Composer(content)),
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
    pub fn new(input: &str) -> SearchQuery {
        let tags = input.split(',').filter_map(Tag::from_search_query).collect();
        let order = input.split(',').filter_map(Order::from_search_query).next().unwrap_or(Order::ByDate);

        SearchQuery { tags: tags, order: order }
    }

    /// Check for emptiness
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Converts the search query to SQL
    pub fn to_sql_query(self) -> String {
        let mut tmp: String = "SELECT * FROM Tracks".into();
        
        if !self.tags.is_empty() {
            tmp.push_str(" WHERE ");
            tmp.push_str(&self.tags.into_iter().map(|x| x.to_sql_query()).collect::<Vec<String>>().join(" AND "));
        }

        tmp.push_str(" ORDER BY ");
        tmp.push_str(&self.order.name());
        tmp.push_str(" DESC");

        tmp
    }
}

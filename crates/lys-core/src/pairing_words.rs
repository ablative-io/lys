//! The words a pairing code is made of (AGENTS-020 R1): two hundred and
//! fifty-six common, short English words, no two of which sound alike, so a
//! person can read four of them aloud over a phone and the one listening
//! writes down the same four. Each byte of a code picks one word, so four
//! bytes from the operating system make one code.
//!
//! Kept beside the handed link (DIRECTORY-112), which is a token a person
//! clicks; this is the list a person says.

/// The words, one per byte value.
pub const WORDS: [&str; 256] = [
    "acorn", "amber", "anchor", "apple", "apron", "javelin", "attic", "autumn", "badge", "bagel",
    "bamboo", "banjo", "barrel", "basket", "beacon", "beetle", "bison", "blanket", "blossom",
    "bottle", "bridge", "broom", "bubble", "bucket", "biscuit", "button", "cabin", "cactus",
    "camel", "candle", "canoe", "canyon", "carpet", "carrot", "castle", "cedar", "cello", "cherry",
    "chimney", "custard", "circus", "clover", "cobalt", "coconut", "coffee", "comet", "compass",
    "copper", "coral", "denim", "cradle", "peacock", "cricket", "crystal", "cupcake", "curtain",
    "daisy", "dolphin", "domino", "donkey", "dragon", "drum", "eagle", "echo", "elbow", "flamingo",
    "engine", "falcon", "feather", "fiddle", "finch", "fossil", "fountain", "garden", "garlic",
    "gecko", "ginger", "glacier", "goblet", "goose", "granite", "guitar", "hammer", "hammock",
    "sailor", "harvest", "hazel", "helmet", "heron", "beaver", "honey", "iceberg", "igloo",
    "island", "ivory", "jacket", "jaguar", "jasmine", "jelly", "jigsaw", "jungle", "kayak",
    "kettle", "kitten", "koala", "ladder", "lantern", "lemon", "leopard", "lettuce", "lizard",
    "llama", "lobster", "kangaroo", "lotus", "magnet", "mango", "marble", "prairie", "parsnip",
    "glove", "gorilla", "mosaic", "muffin", "mushroom", "napkin", "nectar", "needle", "paprika",
    "nutmeg", "oasis", "octopus", "olive", "onion", "orbit", "orchid", "ostrich", "otter",
    "oyster", "oatmeal", "pyramid", "porridge", "panda", "toucan", "pasta", "peanut", "pebble",
    "pelican", "pencil", "penguin", "saffron", "piano", "pickle", "pigeon", "pillow", "pirate",
    "nebula", "magpie", "poodle", "poppy", "potato", "pretzel", "pumpkin", "puppet", "puzzle",
    "quilt", "rabbit", "raccoon", "radish", "rainbow", "raisin", "raven", "ribbon", "river",
    "spinach", "rocket", "rooster", "rubber", "saddle", "salmon", "tortoise", "satchel", "scarf",
    "scooter", "shadow", "sheep", "shovel", "silver", "sketch", "loafer", "snail", "soccer",
    "sofa", "sparrow", "spider", "sponge", "spoon", "squirrel", "stable", "stapler", "sticker",
    "stork", "summer", "sunset", "sweater", "tablet", "clarinet", "teapot", "temple", "tennis",
    "thimble", "thunder", "wombat", "tiger", "toaster", "turnip", "tractor", "trumpet", "tulip",
    "tunnel", "vulture", "turtle", "umbrella", "unicorn", "velvet", "violet", "trombone",
    "volcano", "waffle", "wagon", "walnut", "walrus", "goblin", "wasabi", "harp", "window",
    "snowman", "sapphire", "pudding", "zebra", "zipper", "zucchini", "bronze", "cobweb", "dinghy",
    "flannel", "gravel", "hedgehog", "kiwi", "lagoon", "mammoth", "platypus", "pistol", "quartz",
    "sardine", "lentil",
];

/// The code four bytes name: four words from [`WORDS`], one per byte,
/// separated by single spaces.
#[must_use]
pub fn code_from(bytes: [u8; 4]) -> String {
    bytes
        .iter()
        .map(|byte| WORDS[usize::from(*byte)])
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{WORDS, code_from};

    #[test]
    fn pairing_words_are_distinct_lowercase_short_and_two_hundred_and_fifty_six() {
        let mut seen = std::collections::BTreeSet::new();
        for word in WORDS {
            assert!(
                word.len() >= 3 && word.len() <= 8,
                "{word} is not short enough to say"
            );
            assert!(
                word.bytes().all(|byte| byte.is_ascii_lowercase()),
                "{word} is not lowercase ascii"
            );
            assert!(seen.insert(word), "{word} is listed twice");
        }
        assert_eq!(seen.len(), 256);
    }

    #[test]
    fn pairing_code_takes_one_word_per_byte() {
        assert_eq!(code_from([0, 1, 2, 255]), "acorn amber anchor lentil");
        let code = code_from([7, 7, 7, 7]);
        let words: Vec<&str> = code.split(' ').collect();
        assert_eq!(words, ["autumn"; 4]);
    }
}

use std::collections::HashMap;
use std::thread::sleep;
use std::error::Error;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use thirtyfour::prelude::*;
use lingual::{non_blocking::translate, Lang};
use whatlang::Detector;
use whatlang::Lang as WhatLang;
use futures::{self, lock::Mutex};
use regex;
use tokio;
// 528ffzlg9@mozmail.com
// Cw;-)X42&+w9MD!

#[derive(Debug, PartialEq, Eq, Hash)]
struct LanguageTranslation {
    text: String,
    from_language: Lang,
    to_language: Lang,
}

#[allow(unused)]
struct TranslationDictionary {
    runtime: tokio::runtime::Runtime,
    translations: HashMap<LanguageTranslation, String>,
    language_detection: HashMap<String, Lang>,
}

#[allow(unused)]
impl TranslationDictionary {
    fn new() -> Self {
        TranslationDictionary {
            runtime: tokio::runtime::Runtime::new().unwrap(),
            translations: HashMap::new(),
            language_detection: HashMap::new(),
        }
    }

    async fn find_language(&mut self, text: &str) -> Lang {
        match self.language_detection.get(text) {
            Some(language) => *language,
            None => {
                let language = self.detect_language(text);
                self.language_detection.insert(text.to_string(), language);
                language
            },
        }
    }

    fn detect_language(&mut self, text: &str) -> Lang {
        let allow_list = vec![WhatLang::Nld, WhatLang::Eng];
        let detector = Detector::with_allowlist(allow_list);
        let info = detector.detect(text).unwrap();
        let language = match info.lang() {
            WhatLang::Nld => Lang::Nl,
            WhatLang::Eng => Lang::En,
            _ => panic!("Unknown language"),
        };
        language
    }

    /// this is going to translate both words and sentences
    /// where if the word/sentence is not in the dictionary
    /// it uses the async translate function to translate the word/sentence
    /// 
    /// if the word/sentence is in the dictionary
    /// it will be returned
    fn lookup(&mut self, text: &[String], from_language: Lang, to_language: Lang) -> String {    
        // generate the key for the hash dictionary
        let language_translation = if text.len() == 1 {
            LanguageTranslation {
                text: text[0].clone(),
                from_language,
                to_language,
            }
        } else {
            let sentence = text.join(" ");
            LanguageTranslation {
                text: sentence,
                from_language,
                to_language,
            }
        };
        
        match self.translations.get(&language_translation) {
            Some(translations) => {
                // return the translation
                translations.clone()
            },
            None => {
                // translate the async word
                // add the translation to the dictionary
                // return the translation
                let translated_word = self.runtime.block_on(translate(language_translation.text.clone(), Some(Lang::En), Some(Lang::Nl))).unwrap().text().to_string();
                self.translations.insert(language_translation, translated_word.clone());
                translated_word
            },
        }
    }

    async fn lookup_async(&mut self, text: &[String], from_language: Lang, to_language: Lang) -> String {
        // generate the key for the hash dictionary
        let language_translation = if text.len() == 1 {
            LanguageTranslation {
                text: text[0].clone(),
                from_language,
                to_language,
            }
        } else {
            let sentence = text.join(" ");
            LanguageTranslation {
                text: sentence,
                from_language,
                to_language,
            }
        };
        
        match self.translations.get(&language_translation) {
            Some(translations) => {
                // return the translation
                translations.clone()
            },
            None => {
                // translate the async word
                // add the translation to the dictionary
                // return the translation
                let translated_word = translate(language_translation.text.clone(), Some(from_language), Some(to_language)).await.unwrap().text().to_string();
                self.translations.insert(language_translation, translated_word.clone());
                translated_word
            },
        }    
    }

    fn insert_translation(&mut self, from_text: String, from_language: Lang, to_language: Lang, translated_text: String) {
        let language_translation = LanguageTranslation {
            text: from_text,
            from_language,
            to_language,
        };
    
        // insert the translation into the dictionary but first check if it is not already in the dictionary then remove it
        if self.translations.contains_key(&language_translation) {
            self.translations.remove(&language_translation);
        }
        self.translations.insert(language_translation, translated_text);
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut settings = Settings::load(std::path::Path::new("./settings.json"));
    
    std::env::set_var("WEBDRIVER_GECKO_DRIVER", settings.get("path_to_geckodriver"));
    let mut _driver_process = std::process::Command::new(settings.get("path_to_geckodriver")).spawn().unwrap();
    
    let mut caps = DesiredCapabilities::firefox();
    if settings.headless {
        caps.add_firefox_option("args", vec!["-headless"])?;
    }
    let driver = WebDriver::new("http://localhost:4444", caps).await?;
    login(&driver, settings.email, settings.password).await?;
    while driver.current_url().await?.as_str() != "https://www.duolingo.com/learn" {
        sleep(std::time::Duration::from_millis(100));
    }
    sleep(std::time::Duration::from_secs(2));
    let mut translation_dictionary = Arc::new(Mutex::new(TranslationDictionary::new()));

    let popup = driver.find(By::XPath("//button[@data-test='notification-drawer-no-thanks-button']"))
        .await;
    if popup.is_ok() {
        popup.unwrap().click().await?;
    }
    let hearts = driver.find(By::XPath("//span[@class='_2WjcG _2IhxH _2_xxd']")).await?;
    let amount_of_hearts: u8 = hearts.text().await?.parse().unwrap();
    if amount_of_hearts < 5 {
        hearts.click().await?;
        hearts.click().await?;
        sleep(std::time::Duration::from_millis(100));
        let buttons = driver.find_all(By::XPath("//button[@class='_1N-oo _36Vd3 _16r-S _37iKA']"))
            .await?;
        buttons[1].click().await?;
        solve_challanges(&driver, &mut translation_dictionary).await?;
    }

    do_lessons(&driver, &mut translation_dictionary).await?;
    Ok(())
}

async fn do_lessons(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let lessons = driver.find_all(By::XPath("//div[@class='_31n11 _3DQs0']")).await?;
    let amount_of_lessons_regex = regex::Regex::new(r#"(?m)\d+"#).unwrap();

    for lesson in lessons {
        lesson.click().await?;
        let amount_of_lessons_text = driver.find(By::XPath("//p[@class='_3DPNK']"))
            .await?.text().await?;
        let regex_capture = amount_of_lessons_regex.captures_iter(&amount_of_lessons_text).collect::<Vec<_>>();
        let lessons_done: u8 = regex_capture.get(0).unwrap()[0].parse().unwrap();
        let lessons_total: u8 = regex_capture.get(1).unwrap()[0].parse().unwrap();
        let lessons_left = lessons_total - lessons_done;

        for _ in 0..lessons_left {
            driver.find(By::XPath("//a[@class='_30qMV _2N_A5 _36Vd3 _16r-S KSXIb _2CJe1 _12StQ']")).await?.click().await?;
            println!("Entered lesson {}/{}", lessons_done, lessons_total);
            solve_challanges(&driver, translation_dictionary).await?;
            sleep(std::time::Duration::from_secs(1));
        }
    }
    Ok(())
}

#[allow(unused)]
async fn do_lessons_multi(driver: &WebDriver) -> Result<(), Box<dyn Error + Send + Sync>> {
    let translation_dictionary = Arc::new(tokio::sync::Mutex::new(TranslationDictionary::new()));
    let lessons = driver.find_all(By::XPath("//div[@class='_31n11 _3DQs0']")).await?;
    todo!("do lessons multi");
    Ok(())
}

async fn login(driver: &WebDriver, email: String, password: String) -> Result<(), Box<dyn Error + Send + Sync>> {
    driver.goto("https://www.duolingo.com/").await?;
    driver.find(By::XPath("//button[@data-test='have-account']")).await?.click().await?;
    driver.find(By::XPath("//input[@data-test='email-input']"))
        .await?
        .send_keys(email)
        .await?;
    driver.find(By::XPath("//input[@data-test='password-input']"))
    .await?
        .send_keys(password)
        .await?;
    driver.find(By::XPath("//button[@data-test='register-button']"))
    .await?
    .click()
    .await?;
Ok(())
}

#[derive(Debug)]
enum ChallangeTypes {
    Select,
    Translate,
    Assist,
    Match,
    Name,
    PartialReverseTranslate,
    Cannot,
    Ignore,
}

async fn solve_challanges(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    sleep(std::time::Duration::from_secs(3));  
    loop {
        match get_challange_type(driver).await? {
            ChallangeTypes::Select => solve_select_challange(driver, translation_dictionary).await?,
            ChallangeTypes::Translate => solve_translate_challange(driver, translation_dictionary).await?,
            ChallangeTypes::Assist => solve_assist_challange(driver, translation_dictionary).await?,
            ChallangeTypes::Match => solve_match_challange(driver, translation_dictionary).await?,
            ChallangeTypes::Name => panic!("name is not implemented"),
            ChallangeTypes::PartialReverseTranslate => solve_partial_reverse_translate_challange(driver, translation_dictionary).await?,
            ChallangeTypes::Cannot => panic!("cannot is not implemented"),
            ChallangeTypes::Ignore => {
                sleep(std::time::Duration::from_secs(1));
                println!("Skipping motivaton");
            },
        }
        let next_button = driver.find(By::XPath("//button[@data-test='player-next']")).await?;
        next_button.wait_until().clickable().await?;
        next_button.click().await?;
        sleep(std::time::Duration::from_millis(100));
        if driver.current_url().await?.as_str() == "https://www.duolingo.com/learn" {
            break;
        }
    } 
    Ok(())
}

async fn get_challange_type(driver: &WebDriver) -> Result<ChallangeTypes, Box<dyn Error + Send + Sync>> {
    let challange_type: ChallangeTypes = match driver.find(By::XPath("//div[@class='e4VJZ FQpeZ']")).await {
        Ok(web_element) => {
            let attr = web_element.attr("data-test").await?.unwrap();
            let challange_type_str = attr.trim_start_matches("challenge ");
            let challange_type = match challange_type_str {
                "challenge-translate" => ChallangeTypes::Translate,
                "challenge-select" => ChallangeTypes::Select,
                "challenge-assist" => ChallangeTypes::Assist,
                "challenge-name" => ChallangeTypes::Name,
                "challenge-partialReverseTranslate" => ChallangeTypes::PartialReverseTranslate,
                "challenge-match" => ChallangeTypes::Match,
                "challenge-listen" => ChallangeTypes::Cannot,
                _ => panic!("Unknown challange type: {}", challange_type_str),
            };
            println!("{}", challange_type_str);
            challange_type
        },
        Err(_) => ChallangeTypes::Ignore,
    };
    Ok(challange_type)
}

async fn solve_select_challange(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let challange_header = driver.find(By::XPath("//h1[@data-test='challenge-header']"))
        .await?
        .text()
        .await?;
    let regex = regex::Regex::new(r#"(?m)“([^”]*)”"#).unwrap();
    let challange_text = regex.captures(&challange_header).unwrap()[1].to_string();

    let available_choices: Vec<WebElement> = driver.find_all(By::XPath("//div[@data-test='challenge-choice']")).await?;
    let choice_texts: Vec<_> = available_choices.iter()
        .map(|choice| choice.text())
        .collect();
    let choices_results: Vec<Result<String, WebDriverError>> = futures::future::join_all(choice_texts).await;    
    let choices: Vec<String> = choices_results.into_iter().map(|res| res.unwrap().trim_end_matches(char::is_numeric).trim_end_matches("\n").to_owned()).collect();

    let translated_challange_text = translation_dictionary.lock().await.lookup_async(&[challange_text], Lang::En, Lang::Nl).await;
    let translated_choices = choices.iter()
        .map(|choice| {
            let translation_dictionary = Arc::clone(translation_dictionary);
            let choice = choice.clone();
            async move {
                let mut translation_dictionary = translation_dictionary.lock().await;
                translation_dictionary.lookup_async(&[choice], Lang::En, Lang::Nl).await
            }
        })
        .collect::<Vec<_>>();

    let mut choice_index = 0;
    for (index, choice) in translated_choices.into_iter().enumerate() {
        let choice = choice.await;
        if choice.to_lowercase() == translated_challange_text {
            choice_index = index;
            break;
        }
    }
    available_choices[choice_index].click().await?;
    driver.find(By::XPath("//button[@data-test='player-next']")).await?.click().await?;
    Ok(())
}

async fn solve_assist_challange(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let word = driver.find(By::XPath("//div[@class='_1KUxv _11rtD']")).await?.text().await?;
    let from_lang = translation_dictionary.lock().await.find_language(&word).await;
    let to_lang = if from_lang == Lang::En { Lang::Nl } else { Lang::En };
    println!("Translating: {}", word);
    let translated_word = translation_dictionary.lock().await.lookup_async(&[word], from_lang, to_lang).await;
    println!("Translated: {}", translated_word);
    
    let challange_choices = driver.find_all(By::XPath("//div[@data-test='challenge-choice']")).await?;
    let mut choice_index = 0;
    for (index, choice) in challange_choices.iter().enumerate() {
        let choice_text = choice.text().await?.trim_start_matches(char::is_numeric).trim_start_matches("\n").to_string();
        if choice_text == translated_word {
            choice_index = index;
            break;
        }
    }
    challange_choices[choice_index].click().await?;
    driver.find(By::XPath("//button[@data-test='player-next']")).await?.click().await?;
    Ok(())
}

async fn solve_translate_challange(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let challange_header = driver.find(By::XPath("//h1[@data-test='challenge-header']"))
        .await?
        .text()
        .await?;
    let from_lang = match challange_header.to_lowercase().as_str() {
        "write this in english" => Lang::Nl,
        "write this in dutch" => Lang::En,
        _ => panic!("Unknown language"),
    };
    let to_lang = if from_lang == Lang::En { Lang::Nl } else { Lang::En };
    println!("From language: {:?}, To language: {:?}", from_lang, to_lang);
    let sentence = driver.find(By::XPath("//span[@class='g-kCu']")).await?.text().await?;
    println!("Translating: {}", sentence);
    let translated_sentence = translation_dictionary.lock().await.lookup_async(&[sentence], from_lang, to_lang).await;
    println!("Translated: {}", translated_sentence);
    let sentence_input = driver.find(By::XPath("//body")).await?;
    for word in translated_sentence.split(" ") {
        for char in word.chars() {
            sentence_input.send_keys(format!("{}", char)).await?;
            sleep(std::time::Duration::from_millis(10));
        }
        sentence_input.send_keys(" ").await?;
    }
    sentence_input.send_keys(" " + Key::Enter).await?;
    if driver.find(By::XPath("//div[@data-test='blame blame-incorrect']")).await.is_ok() {
        let duolingo_translation = driver.find(By::XPath("//div[@class='_1UqAr _3Qruy']")).await?.text().await?;
        println!("Duolingo translation: {}", duolingo_translation);
    }
    Ok(())
}

async fn solve_match_challange(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cards = driver.find_all(By::XPath("//span[@data-test='challenge-tap-token-text']")).await?;
    // take only half of the card because the amount of cards are always equal
    // tranlate them and match them use the indexes and send the key presses
    let mut card_texts = Vec::new();
    for card in &cards{
        card_texts.push(card.text().await?.to_owned());
    }
    let half = card_texts.len() / 2;
    let from_cards = &card_texts[..half];
    let to_cards = &card_texts[half..];
    // translate the from_cards to compare them with the to_cards
    let mut translated_from_cards = Vec::new();
    from_cards.iter().for_each(|card| {
        let translation_dictionary = Arc::clone(translation_dictionary);
        let card = card.clone();
        translated_from_cards.push(async move {
            let mut translation_dictionary = translation_dictionary.lock().await;
            translation_dictionary.lookup_async(&[card], Lang::En, Lang::Nl).await
        });
    });
    // find out which from_card indexes matches with to_card and get the indexes
    let mut indexes = Vec::new();
    for (index, translated_from_card) in translated_from_cards.into_iter().enumerate() {
        let translated_from_card = translated_from_card.await;
        for (to_index, to_card) in to_cards.iter().enumerate() {
            if translated_from_card.to_lowercase() == *to_card.to_lowercase() {
                indexes.push((index, to_index));
                break;
            }
        }
    }
    // send key presses to the body
    let body = driver.find(By::XPath("//body")).await?;
    for (from_index, to_index) in indexes {
        let key_presses1 = format!("{}", from_index + 1);
        let key_presses2 = format!("{}", to_index + 1 + half);
        println!("Sending key presses: {}, {}", key_presses1, key_presses2);
        body.send_keys(key_presses1).await?;
        body.send_keys(key_presses2).await?;
        sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}

async fn solve_partial_reverse_translate_challange(driver: &WebDriver, translation_dictionary: &mut Arc<Mutex<TranslationDictionary>>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let sentence = driver.find(By::XPath("//span[@class='g-kCu']")).await?.text().await?;
    let from_lang = translation_dictionary.lock().await.find_language(&sentence).await;
    let to_lang = if from_lang == Lang::En { Lang::Nl } else { Lang::En };
    let _translated_sentence = translation_dictionary.lock().await.lookup_async(&[sentence], from_lang, to_lang).await;
    // this above is only to get the translation in the dictionary
    let rest_sentence = driver.find(By::XPath("//span[@class='_31xxw _2eX9t _1vqO5']")).await?.text().await?;
    let text_entry = driver.find(By::XPath("//label[@class='_1fYGK _2FKqf _2ti2i']")).await?;
    text_entry.focus().await?;
    println!("1");
    for char in rest_sentence.chars() {
        text_entry.send_keys(format!("{}", char)).await?;
        sleep(std::time::Duration::from_millis(10));
    }
    text_entry.send_keys(" " + Key::Enter).await?;
    println!("2");
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    pub headless: bool,
    pub email: String,
    pub password: String,
    pub path_to_geckodriver: String
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            headless: false,
            email: String::new(),
            password: String::new(),
            path_to_geckodriver: String::new()
        }
    }

    pub fn save(&self, fp: &std::path::Path) {
        let settings = serde_json::to_string(self).unwrap();
        std::fs::write(fp, settings).unwrap();
    }

    pub fn load(fp: &std::path::Path) -> Settings {
        let settings = std::fs::read_to_string(fp).unwrap();
        let settings: Settings = serde_json::from_str(&settings).unwrap();
        settings
    }

    pub fn get(&self, key: &str) -> String {
        match key {
            "headless" => self.headless.to_string(),
            "email" => self.email.clone(),
            "password" => self.password.clone(),
            "path_to_geckodriver" => self.path_to_geckodriver.clone(),
            _ => panic!("Unknown key: {}", key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_translation_dictionary() {
        let mut translation_dictionary = TranslationDictionary::new();

        let words: Vec<String> = "Have a good night sleep and goodnight".split(" ").map(|word| word.to_string()).collect();
        let from_language = Lang::En;
        let to_language = Lang::Nl;
        
        let start = std::time::Instant::now();
        let translation = translation_dictionary.lookup(&words, from_language, to_language);
        let end = std::time::Instant::now();
        println!("Time to translate: {:?}", end - start);
        assert_eq!(translation, "Heb een goede nachtrust en welterusten");

        // now the translation should be in the dictionary and should take less time
        let start = std::time::Instant::now();
        let translation = translation_dictionary.lookup(&words, from_language, to_language);
        let end = std::time::Instant::now();
        println!("Time to translate: {:?}", end - start);
        assert_eq!(translation, "Heb een goede nachtrust en welterusten");
    }

    #[test]
    fn test_load_settings() {
        let settings = Settings::load(std::path::Path::new("./settings.json"));
        assert_eq!(settings.headless, false);
        assert_eq!(settings.password, "Cw;-)X42&+w9MD!");
    }
    
    #[test]
    fn test_regex() {
        let regex = regex::Regex::new(r#"(?m)“([^”]*)”"#).unwrap();
        let text = r#"Which one of these is “the girl”?"#;
        let capture = regex.captures(text).unwrap();
        println!("{:?}", &capture[1]);
    }

}

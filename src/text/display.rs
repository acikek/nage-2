use std::{fmt::{Display, Debug}, time::Duration};

use anyhow::Result;
use crossterm::style::Stylize;
use serde::{Deserialize, Serialize};
use snailshell::{snailprint_s, snailprint_d};
use strum::{EnumString, Display};

use crate::loading::base::{ContentFile, Contents};

use super::{templating::{TemplatableValue, TemplatableString}, context::TextContext};

#[derive(Deserialize, Serialize, Display, Debug, PartialEq, Clone, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
/// Represents how text should be formatted disregarding its contents.
pub enum TextMode {
	#[serde(alias = "dialog")]
	/// Wraps text in quotes.
	Dialogue,
	/// Returns text as-is.
	Action,
	/// Prefixes text with a quote character.
	System
}

impl Default for TextMode {
	fn default() -> Self {
		Self::Dialogue
	}
}

impl TextMode {
	/// Formats a [`String`] based on the selected text mode.
	/// 
	/// See [`Mode`] types to view how a text mode will format content.
	pub fn format(&self, text: &str) -> String {
		use TextMode::*;
		match self {
			Dialogue => format!("\"{text}\""),
			Action => text.to_owned(),
			System => format!("{} {text}", "▐".dark_grey())
		}
	}
}

/// The speed at which text should be printed.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TextSpeed {
	/// The amount of milliseconds to wait between each character.
	Delay(TemplatableValue<usize>),
	/// The rate, in characters per second, at which the text is printed.
	Rate(TemplatableValue<f32>),
	/// The amount of milliseconds that the text should take to print regardless of content length.
	Duration(TemplatableValue<usize>)
}

impl Default for TextSpeed {
	fn default() -> Self {
		TextSpeed::Rate(TemplatableValue::value(200.0))
	}
}

impl TextSpeed {
	/// Calculates or returns the rate in charatcers per second
	/// to be used in [`snailprint_s`].
	/// 
	/// If this object is [`Rate`](TextSpeed::Rate), returns the contained value.
	/// If it is [`Delay`](TextSpeed::Delay), calculates the rate with `(1.0 / delay) * 1000.0`.
	pub fn rate(&self, context: &TextContext) -> Result<f32> {
		use TextSpeed::*;
		let result = match &self {
			Rate(rate) => rate.get_value(context)?,
			Delay(delay) => 1.0 / delay.get_value(context)? as f32 * 1000.0,
			_ => unreachable!()
		};
		Ok(result)
	}

	/// Snailprints some content.
	/// 
	/// If the object is [`Rate`](TextSpeed::Rate) or [`Delay`](TextSpeed::Delay), uses [`snailprint_s`]
	/// with the rate returned from [`TextSpeed::rate`].
	/// 
	/// Otherwise, if the object is [`Duration`](TextSpeed::Duration), uses [`snailprint_d`] with the
	/// specified length of time.
	pub fn print<T>(&self, content: &T, context: &TextContext) -> Result<()> where T: Display {
		let result = match &self {
			TextSpeed::Duration(duration) => snailprint_d(content, duration.get_value(context)? as f32 / 1000.0),
			_ => snailprint_s(content, self.rate(context)?)
		};
		Ok(result)
	}
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
/// A formattable piece of text.
pub struct Text {
	#[serde(rename = "text")]
	/// The unformatted text content.
	pub content: TemplatableString,
	#[serde(default)]
	/// The mode in which the text content should be formatted upon retrieval.
	pub mode: TemplatableValue<TextMode>,
	pub speed: Option<TextSpeed>,
	pub newline: Option<TemplatableValue<bool>>,
	pub wait: Option<TemplatableValue<u64>>
}

/// An ordered list of text objects.
pub type TextLines = Vec<Text>;
/// An ordered list of text objects with a flag representing whether the last entry was of the same [`TextMode`].
pub type SeparatedTextLines<'a> = Vec<(bool, &'a Text)>;

pub type TranslationFile = ContentFile<String>;
pub type Translations = Contents<String>;

impl Text {
	/// Retrieves text content with [`TemplatableString::fill`] and formats it based on the [`TextMode`].
	pub fn get(&self, context: &TextContext) -> Result<String> {
		Ok(self.mode.get_value(context)?.format(&self.content.fill(context)?))
	}

	/// Formats and snailprints text based on its [`TextSpeed`]. 
	/// 
	/// If the text object does not contain a `speed` field, defaults to the provided config settings.
	pub fn print(&self, context: &TextContext) -> Result<()> {
		let speed = self.speed.as_ref().unwrap_or(&context.config.settings.speed);
		speed.print(&self.get(context)?, context)?;
		if let Some(wait) = &self.wait {
			std::thread::sleep(Duration::from_millis(wait.get_value(context)?));
		}
		Ok(())
	}

	/// Whether a newline should be printed before this line.
	/// Uses the `newline` key, otherwise defaulting to comparing the [`TextMode`] between this and the previous line, if any.
	fn newline(&self, previous: Option<&Text>, context: &TextContext) -> Result<bool> {
		self.newline.as_ref()
			.map(|nl| nl.get_value(context))
    		.unwrap_or(previous
				.map(|line| Ok(self.mode.get_value(context)? != line.mode.get_value(context)?))
				.unwrap_or(Ok(false))
			)
	}

	/// Calculates some [`SeparatedTextLines`] based on some text lines.
	fn get_separated_lines<'a>(lines: &'a TextLines, context: &TextContext) -> Result<SeparatedTextLines<'a>> {
		lines.iter().enumerate()
    		.map(|(index, line)| Ok((line.newline(index.checked_sub(1).map(|i| &lines[i]), context)?, line)))
    		.collect()
	}

	/// Formats and separates text lines and prints them sequentially.
	pub fn print_lines(lines: &TextLines, context: &TextContext) -> Result<()> {
		for (newline, line) in Self::get_separated_lines(lines, context)? {
			if newline {
				println!();
			}
			line.print(context)?;
		}
		Ok(())
	}

	/// Calls [`Text::print_lines`] and prints a newline at the end.
	pub fn print_lines_nl(lines: &TextLines, context: &TextContext) -> Result<()> {
		Self::print_lines(lines, context)?;
		println!();
		Ok(())
	}
}
use crate::PredictArgs;
use anyhow::Result;
use either::Either;
use itertools::Itertools;
use rayon::prelude::*;
use tangram_core::predict::{PredictInput, PredictInputValue, PredictOptions};
use tangram_zip::zip;

const PREDICT_CHUNK_SIZE: usize = 100;

pub fn predict(args: PredictArgs) -> Result<()> {
	let bytes = std::fs::read(&args.model)?;
	let model = tangram_model::from_bytes(&bytes)?;
	let target_column_name = match model.inner() {
		tangram_model::ModelInnerReader::Regressor(regressor) => {
			regressor.read().target_column_name()
		}
		tangram_model::ModelInnerReader::BinaryClassifier(binary_classifier) => {
			binary_classifier.read().target_column_name()
		}
		tangram_model::ModelInnerReader::MulticlassClassifier(multiclass_classifier) => {
			multiclass_classifier.read().target_column_name()
		}
	};
	let model = tangram_core::predict::Model::from(model);
	let mut options = PredictOptions {
		compute_feature_contributions: false,
		..Default::default()
	};
	if let Some(threshold) = args.threshold {
		options.threshold = threshold;
	}
	let reader = match args.file {
		Some(path) => Either::Left(std::fs::File::open(path)?),
		None => Either::Right(std::io::stdin()),
	};
	let mut reader = csv::Reader::from_reader(reader);
	let writer = match args.output {
		Some(path) => Either::Left(std::fs::File::create(path)?),
		None => Either::Right(std::io::stdout()),
	};
	let mut writer = csv::Writer::from_writer(writer);
	let should_output_probabilies = args.probabilities.unwrap_or(false);
	match &model.inner {
		tangram_core::predict::ModelInner::Regressor(_) => {
			writer.write_record(&[target_column_name])?;
		}
		tangram_core::predict::ModelInner::BinaryClassifier(model) => {
			if should_output_probabilies {
				writer.write_record(&[
					model.positive_class.to_string(),
					model.negative_class.to_string(),
				])?;
			} else {
				writer.write_record(&[target_column_name])?;
			}
		}
		tangram_core::predict::ModelInner::MulticlassClassifier(model) => {
			if should_output_probabilies {
				writer.write_record(
					&model
						.classes
						.iter()
						.map(|class| class.to_string())
						.collect::<Vec<_>>(),
				)?;
			} else {
				writer.write_record(&[target_column_name])?;
			}
		}
	};
	let header = reader.headers()?.to_owned();
	let mut record_chunks: Vec<Vec<_>> = Vec::new();
	for record_chunk in &reader.records().chunks(PREDICT_CHUNK_SIZE) {
		let record_chunk = record_chunk.collect();
		record_chunks.push(record_chunk);
	}
	let output_chunks: Vec<Result<_, _>> = record_chunks
		.into_par_iter()
		.map(|records| {
			let input: Result<Vec<PredictInput>, _> = records
				.into_iter()
				.map(|record| -> Result<PredictInput> {
					let record = record?;
					let input = zip!(header.iter(), record.into_iter())
						.map(|(column_name, value)| {
							(
								column_name.to_owned(),
								PredictInputValue::String(value.to_owned()),
							)
						})
						.collect();
					Ok(PredictInput(input))
				})
				.collect();
			input.map(|input| tangram_core::predict::predict(&model, &input, &options))
		})
		.collect();
	for outputs in output_chunks {
		for output in outputs? {
			let output = match output {
				tangram_core::predict::PredictOutput::Regression(output) => {
					vec![output.value.to_string()]
				}
				tangram_core::predict::PredictOutput::BinaryClassification(output) => {
					let model = match &model.inner {
						tangram_core::predict::ModelInner::BinaryClassifier(model) => model,
						_ => {
							unreachable!()
						}
					};
					let class_name = output.class_name;
					let positive_class_probability = if class_name == model.positive_class {
						output.probability
					} else {
						1.0 - output.probability
					};
					let negative_class_probability = 1.0 - positive_class_probability;
					if should_output_probabilies {
						vec![
							positive_class_probability.to_string(),
							negative_class_probability.to_string(),
						]
					} else {
						vec![class_name]
					}
				}
				tangram_core::predict::PredictOutput::MulticlassClassification(output) => {
					if should_output_probabilies {
						output
							.probabilities
							.iter()
							.map(|(_, probability)| probability.to_string())
							.collect()
					} else {
						vec![output.class_name]
					}
				}
			};
			writer.write_record(&output)?;
		}
	}
	Ok(())
}

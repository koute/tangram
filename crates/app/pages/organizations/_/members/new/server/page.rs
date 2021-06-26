use pinwheel::prelude::*;
use tangram_app_layouts::{
	app_layout::{AppLayout, AppLayoutInfo},
	document::Document,
};
use tangram_ui as ui;

pub struct Page {
	pub app_layout_info: AppLayoutInfo,
	pub error: Option<String>,
}

impl Component for Page {
	fn into_node(self) -> Node {
		Document::new()
			.child(
				AppLayout::new(self.app_layout_info).child(
					ui::S1::new().child(ui::H1::new().child("Invite")).child(
						ui::Form::new()
							.post(Some(true))
							.child(
								ui::TextField::new()
									.label("Email".to_owned())
									.name("email".to_owned()),
							)
							.child(
								ui::CheckboxField::new()
									.label("Admin".to_owned())
									.name("is_admin".to_owned()),
							)
							.child(
								ui::Button::new()
									.button_type(Some(ui::ButtonType::Submit))
									.child("Invite"),
							),
					),
				),
			)
			.into_node()
	}
}

use crate::app::SwitchAppsState;
use crate::utils::{check_error, get_moinitor_rect, is_win11};

use anyhow::{Context, Result};
use windows::Win32::{
	Foundation::{COLORREF, HWND, POINT, RECT, SIZE},
	Graphics::{
		Gdi::{
			CreateCompatibleBitmap, CreateCompatibleDC, CreateRoundRectRgn, CreateSolidBrush,
			DeleteDC, DeleteObject, FillRect, FillRgn, GetDC, ReleaseDC, SelectClipRgn,
			SelectObject, SetStretchBltMode, StretchBlt, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION,
			HALFTONE, HBITMAP, HDC, HPALETTE, SRCCOPY,
		},
		GdiPlus::{
			FillModeAlternate, GdipAddPathArc, GdipClosePathFigure, GdipCreateBitmapFromHBITMAP,
			GdipCreateFromHDC, GdipCreatePath, GdipCreatePen1, GdipDeleteBrush, GdipDeleteGraphics,
			GdipDeletePath, GdipDeletePen, GdipDisposeImage, GdipDrawImageRect, GdipFillPath,
			GdipFillRectangle, GdipGetPenBrushFill, GdipSetInterpolationMode, GdipSetSmoothingMode,
			GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpBitmap, GpBrush, GpGraphics,
			GpImage, GpPath, GpPen, InterpolationModeHighQualityBicubic, SmoothingModeAntiAlias,
			Unit,
		},
	},
	UI::{
		HiDpi::GetDpiForWindow,
		Input::KeyboardAndMouse::SetFocus,
		WindowsAndMessaging::{
			DrawIconEx, GetCursorPos, ShowWindow, UpdateLayeredWindow, DI_NORMAL, SW_HIDE, SW_SHOW,
			ULW_ALPHA,
		},
	},
};

// window
pub const WINDOW_BASE_RADIUS: i32 = 15;
pub const WINDOW_PADDING_SIZE: i32 = 15;
pub const WINDOW_BACKGROUND_COLOR: u32 = 0xd0e5f4;

// icons
pub const ICONS_BASE_RADIUS: i32 = 8;
pub const ICONS_BASE_SIZE: i32 = 48;
pub const ICONS_HOVER_COLOR: u32 = 0xA6DAFF;
pub const ICONS_HOVER_RADIUS: i32 = 10;
pub const ICONS_PADDING_SIZE: i32 = 10;

// system
pub const SYS_ROW_LENGTH: i32 = 10;
pub const SYS_SCALE_FACTOR: i32 = 6;

// GDI Antialiasing Painter
pub struct GdiAAPainter {
	token: usize,
	hwnd: HWND,
	hdc_screen: HDC,
	rounded_corner: bool,
	show: bool,
}

impl GdiAAPainter {
	pub fn new(hwnd: HWND) -> Result<Self> {
		let startup_input = GdiplusStartupInput {
			GdiplusVersion: 1,
			..Default::default()
		};
		let mut token: usize = 0;
		check_error(|| unsafe { GdiplusStartup(&mut token, &startup_input, std::ptr::null_mut()) })
			.context("Failed to initialize GDI+")?;

		let hdc_screen = unsafe { GetDC(Some(hwnd)) };
		let rounded_corner = is_win11();

		Ok(Self {
			token,
			hwnd,
			hdc_screen,
			rounded_corner,
			show: false,
		})
	}

	pub fn paint(&mut self, state: &SwitchAppsState) {
		let dpi_scale = get_dpi_scale(self.hwnd);
		let icon_size_max = (ICONS_BASE_SIZE as f64 * dpi_scale) as i32;
		let border_size = (WINDOW_PADDING_SIZE as f64 * dpi_scale) as i32;
		let icon_border = (ICONS_PADDING_SIZE as f64 * dpi_scale) as i32;

		let Coordinate {
			x,
			y,
			width,
			height,
			icon_size,
			item_size,
		} = Coordinate::new(
			state.apps.len() as i32,
			icon_size_max,
			border_size,
			icon_border,
		);

		let background_corner_radius = if self.rounded_corner {
			scale_px(WINDOW_BASE_RADIUS, dpi_scale)
		} else {
			0
		};
		let hover_corner_radius = if self.rounded_corner {
			scale_px(ICONS_HOVER_RADIUS, dpi_scale)
		} else {
			0
		};
		let icon_corner_radius = scale_px(ICONS_BASE_RADIUS, dpi_scale);

		let hwnd = self.hwnd;
		let hdc_screen = self.hdc_screen;

		let (fg_color, bg_color) = theme_color();

		unsafe {
			let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
			let bitmap_mem = CreateCompatibleBitmap(hdc_screen, width, height);
			SelectObject(hdc_mem, bitmap_mem.into());

			let mut graphics = GpGraphics::default();
			let mut graphics_ptr: *mut GpGraphics = &mut graphics;
			GdipCreateFromHDC(hdc_mem, &mut graphics_ptr as _);
			GdipSetSmoothingMode(graphics_ptr, SmoothingModeAntiAlias);
			GdipSetInterpolationMode(graphics_ptr, InterpolationModeHighQualityBicubic);

			let mut bg_pen = GpPen::default();
			let mut bg_pen_ptr: *mut GpPen = &mut bg_pen;
			GdipCreatePen1(bg_color | 0xff000000, 0.0, Unit(0), &mut bg_pen_ptr);

			let mut bg_brush = GpBrush::default();
			let mut bg_brush_ptr: *mut GpBrush = &mut bg_brush;
			GdipGetPenBrushFill(bg_pen_ptr, &mut bg_brush_ptr as _);

			if self.rounded_corner {
				draw_round_rect(
					graphics_ptr,
					bg_brush_ptr,
					0.0,
					0.0,
					width as f32,
					height as f32,
					background_corner_radius as f32,
				);
			} else {
				GdipFillRectangle(
					graphics_ptr,
					bg_brush_ptr,
					0.0,
					0.0,
					width as f32,
					height as f32,
				);
			}

			let (columns, rows) = grid_dimensions(state.apps.len() as i32);
			let icons_width = item_size * columns;
			let icons_height = item_size * rows;
			let bitmap_icons = draw_icons(
				state,
				hdc_screen,
				icon_size,
				icon_border,
				icons_width,
				icons_height,
				hover_corner_radius,
				icon_corner_radius,
				fg_color,
				bg_color,
			);

			let mut bitmap = GpBitmap::default();
			let mut bitmap_ptr: *mut GpBitmap = &mut bitmap as _;
			GdipCreateBitmapFromHBITMAP(bitmap_icons, HPALETTE::default(), &mut bitmap_ptr as _);

			let image_ptr: *mut GpImage = bitmap_ptr as *mut GpImage;
			GdipDrawImageRect(
				graphics_ptr,
				image_ptr,
				border_size as f32,
				border_size as f32,
				icons_width as f32,
				icons_height as f32,
			);

			let blend = BLENDFUNCTION {
				BlendOp: AC_SRC_OVER as _,
				SourceConstantAlpha: 255,
				AlphaFormat: AC_SRC_ALPHA as _,
				..Default::default()
			};
			let _ = UpdateLayeredWindow(
				hwnd,
				Some(hdc_screen),
				Some(&POINT { x, y }),
				Some(&SIZE {
					cx: width,
					cy: height,
				}),
				Some(hdc_mem),
				Some(&POINT::default()),
				COLORREF(0),
				Some(&blend),
				ULW_ALPHA,
			);

			GdipDisposeImage(image_ptr);
			GdipDeleteBrush(bg_brush_ptr);
			GdipDeletePen(bg_pen_ptr);
			GdipDeleteGraphics(graphics_ptr);

			let _ = DeleteObject(bitmap_icons.into());
			let _ = DeleteObject(bitmap_mem.into());
			let _ = DeleteDC(hdc_mem);
		}

		if self.show {
			return;
		}
		unsafe {
			let _ = ShowWindow(self.hwnd, SW_SHOW);
			let _ = SetFocus(Some(self.hwnd));
		}
		self.show = true;
	}

	pub fn unpaint(&mut self, _state: SwitchAppsState) {
		unsafe {
			let _ = ShowWindow(self.hwnd, SW_HIDE);
		}
		self.show = false;
	}

	pub fn find_clicked_app_index(&self, state: &SwitchAppsState) -> Option<usize> {
		let cursor_pos = unsafe {
			let mut pos = POINT::default();
			let _ = GetCursorPos(&mut pos);
			pos
		};

		let dpi_scale = get_dpi_scale(self.hwnd);
		let icon_size_max = (ICONS_BASE_SIZE as f64 * dpi_scale) as i32;
		let border_size = (WINDOW_PADDING_SIZE as f64 * dpi_scale) as i32;
		let icon_border = (ICONS_PADDING_SIZE as f64 * dpi_scale) as i32;

		let Coordinate {
			x, y, item_size, ..
		} = Coordinate::new(
			state.apps.len() as i32,
			icon_size_max,
			border_size,
			icon_border,
		);

		let xpos = cursor_pos.x - x;
		let ypos = cursor_pos.y - y;

		for (i, _) in state.apps.iter().enumerate() {
			let i = i as i32;
			let cx = border_size + item_size * (i % SYS_ROW_LENGTH);
			let cy = border_size + item_size * (i / SYS_ROW_LENGTH);
			if xpos >= cx && xpos < cx + item_size && ypos >= cy && ypos < cy + item_size {
				return Some(i as usize);
			}
		}
		None
	}
}

impl Drop for GdiAAPainter {
	fn drop(&mut self) {
		unsafe {
			ReleaseDC(Some(self.hwnd), self.hdc_screen);
			GdiplusShutdown(self.token);
		}
	}
}

const fn theme_color() -> (u32, u32) {
	(ICONS_HOVER_COLOR, WINDOW_BACKGROUND_COLOR)
}

unsafe fn draw_round_rect(
	graphic_ptr: *mut GpGraphics,
	brush_ptr: *mut GpBrush,
	left: f32,
	top: f32,
	right: f32,
	bottom: f32,
	corner_radius: f32,
) {
	unsafe {
		let path_ptr = create_round_rect_path(left, top, right, bottom, corner_radius);
		GdipFillPath(graphic_ptr, brush_ptr, path_ptr);
		GdipDeletePath(path_ptr);
	}
}

unsafe fn create_round_rect_path(
	left: f32,
	top: f32,
	right: f32,
	bottom: f32,
	corner_radius: f32,
) -> *mut GpPath {
	unsafe {
		let mut path_ptr: *mut GpPath = std::ptr::null_mut();
		GdipCreatePath(FillModeAlternate, &mut path_ptr as _);
		GdipAddPathArc(
			path_ptr,
			left,
			top,
			corner_radius,
			corner_radius,
			180.0,
			90.0,
		);
		GdipAddPathArc(
			path_ptr,
			right - corner_radius,
			top,
			corner_radius,
			corner_radius,
			270.0,
			90.0,
		);
		GdipAddPathArc(
			path_ptr,
			right - corner_radius,
			bottom - corner_radius,
			corner_radius,
			corner_radius,
			0.0,
			90.0,
		);
		GdipAddPathArc(
			path_ptr,
			left,
			bottom - corner_radius,
			corner_radius,
			corner_radius,
			90.0,
			90.0,
		);
		GdipClosePathFigure(path_ptr);
		path_ptr
	}
}

#[allow(clippy::too_many_arguments)]
fn draw_icons(
	state: &SwitchAppsState,
	hdc_screen: HDC,
	icon_size: i32,
	icon_border: i32,
	width: i32,
	height: i32,
	hover_corner_radius: i32,
	icon_corner_radius: i32,
	fg_color: u32,
	bg_color: u32,
) -> HBITMAP {
	let scaled_width = width * SYS_SCALE_FACTOR;
	let scaled_height = height * SYS_SCALE_FACTOR;
	let scaled_hover_corner_radius = hover_corner_radius * SYS_SCALE_FACTOR;
	let scaled_icon_corner_radius = icon_corner_radius * SYS_SCALE_FACTOR;
	let scaled_border_size = icon_border * SYS_SCALE_FACTOR;
	let scaled_icon_inner_size = icon_size * SYS_SCALE_FACTOR;
	let scaled_icon_outer_size = scaled_icon_inner_size + scaled_border_size * 2;

	unsafe {
		let hdc_tmp = CreateCompatibleDC(Some(hdc_screen));
		let bitmap_tmp = CreateCompatibleBitmap(hdc_screen, width, height);
		SelectObject(hdc_tmp, bitmap_tmp.into());

		let hdc_scaled = CreateCompatibleDC(Some(hdc_screen));
		let bitmap_scaled = CreateCompatibleBitmap(hdc_screen, scaled_width, scaled_height);
		SelectObject(hdc_scaled, bitmap_scaled.into());

		let bg_color_bgr =
			((bg_color & 0x0000FF) << 16) | (bg_color & 0x00FF00) | ((bg_color & 0xFF0000) >> 16);
		let fg_color_bgr =
			((fg_color & 0x0000FF) << 16) | (fg_color & 0x00FF00) | ((fg_color & 0xFF0000) >> 16);

		let fg_brush = CreateSolidBrush(COLORREF(fg_color_bgr));
		let bg_brush = CreateSolidBrush(COLORREF(bg_color_bgr));

		let rect = RECT {
			left: 0,
			top: 0,
			right: scaled_width,
			bottom: scaled_height,
		};

		FillRect(hdc_scaled, &rect, bg_brush);

		let mut highlighted_indices = vec![state.index];
		if let Some(hover_index) = state.hover_index {
			if hover_index != state.index {
				highlighted_indices.push(hover_index);
			}
		}

		for highlighted_index in highlighted_indices {
			let highlighted_index = highlighted_index as i32;
			let highlighted_col = highlighted_index % SYS_ROW_LENGTH;
			let highlighted_row = highlighted_index / SYS_ROW_LENGTH;
			let highlighted_left = scaled_icon_outer_size * highlighted_col;
			let highlighted_top = scaled_icon_outer_size * highlighted_row;
			let highlighted_rgn = CreateRoundRectRgn(
				highlighted_left,
				highlighted_top,
				highlighted_left + scaled_icon_outer_size,
				highlighted_top + scaled_icon_outer_size,
				scaled_hover_corner_radius,
				scaled_hover_corner_radius,
			);
			let _ = FillRgn(hdc_scaled, highlighted_rgn, fg_brush);
			let _ = DeleteObject(highlighted_rgn.into());
		}

		for (i, (icon, _)) in state.apps.iter().enumerate() {
			let i = i as i32;
			let col = i % SYS_ROW_LENGTH;
			let row = i / SYS_ROW_LENGTH;
			let item_left = scaled_icon_outer_size * col;
			let item_top = scaled_icon_outer_size * row;

			let cx = scaled_border_size + item_left;
			let cy = scaled_border_size + item_top;
			let icon_clip_rgn = CreateRoundRectRgn(
				cx,
				cy,
				cx + scaled_icon_inner_size,
				cy + scaled_icon_inner_size,
				scaled_icon_corner_radius,
				scaled_icon_corner_radius,
			);
			SelectClipRgn(hdc_scaled, Some(icon_clip_rgn));
			let _ = DrawIconEx(
				hdc_scaled,
				cx,
				cy,
				*icon,
				scaled_icon_inner_size,
				scaled_icon_inner_size,
				0,
				None,
				DI_NORMAL,
			);
			SelectClipRgn(hdc_scaled, None);
			let _ = DeleteObject(icon_clip_rgn.into());
		}

		SetStretchBltMode(hdc_tmp, HALFTONE);
		let _ = StretchBlt(
			hdc_tmp,
			0,
			0,
			width,
			height,
			Some(hdc_scaled),
			0,
			0,
			scaled_width,
			scaled_height,
			SRCCOPY,
		);

		let _ = DeleteObject(fg_brush.into());
		let _ = DeleteObject(bg_brush.into());
		let _ = DeleteObject(bitmap_scaled.into());
		let _ = DeleteDC(hdc_scaled);
		let _ = DeleteDC(hdc_tmp);

		bitmap_tmp
	}
}

fn get_dpi_scale(hwnd: HWND) -> f64 {
	unsafe {
		let dpi = GetDpiForWindow(hwnd);
		if dpi == 0 {
			1.0
		} else {
			dpi as f64 / 96.0
		}
	}
}

fn scale_px(value: i32, dpi_scale: f64) -> i32 {
	(value as f64 * dpi_scale) as i32
}

fn grid_dimensions(num_apps: i32) -> (i32, i32) {
	let columns = num_apps.min(SYS_ROW_LENGTH);
	let rows = (num_apps + SYS_ROW_LENGTH - 1) / SYS_ROW_LENGTH;
	(columns, rows)
}

struct Coordinate {
	x: i32,
	y: i32,
	width: i32,
	height: i32,
	icon_size: i32,
	item_size: i32,
}

impl Coordinate {
	fn new(num_apps: i32, icon_size_max: i32, border_size: i32, icon_border: i32) -> Self {
		let monitor_rect = get_moinitor_rect();
		let monitor_width = monitor_rect.right - monitor_rect.left;
		let monitor_height = monitor_rect.bottom - monitor_rect.top;

		let (columns, rows) = grid_dimensions(num_apps);
		let icon_size_by_width = (monitor_width - 2 * border_size) / columns - icon_border * 2;
		let icon_size_by_height = (monitor_height - 2 * border_size) / rows - icon_border * 2;
		let icon_size = icon_size_by_width
			.min(icon_size_by_height)
			.min(icon_size_max);

		let item_size = icon_size + icon_border * 2;

		let width = item_size * columns + border_size * 2;
		let height = item_size * rows + border_size * 2;
		let x = monitor_rect.left + (monitor_width - width) / 2;
		let y = monitor_rect.top + (monitor_height - height) / 2;

		Self {
			x,
			y,
			width,
			height,
			icon_size,
			item_size,
		}
	}
}

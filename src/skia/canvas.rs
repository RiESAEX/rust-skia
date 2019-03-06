use std::mem;
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;
use std::slice;
use std::ffi::CString;
use crate::graphics;
use crate::prelude::*;
use crate::skia::{
    IRect,
    QuickReject,
    Region,
    RRect,
    ClipOp,
    Point,
    scalar,
    Vector,
    Image,
    ImageFilter,
    Rect,
    IPoint,
    Surface,
    Bitmap,
    AlphaType,
    ColorType,
    ISize,
    SurfaceProps,
    ImageInfo,
    Path,
    Paint,
    Color,
    Matrix,
    BlendMode,
    Font,
    TextEncoding,
    Picture,
    Vertices,
    VerticesBone,
    Data
};
use rust_skia::{
    C_SkAutoCanvasRestore_destruct,
    SkAutoCanvasRestore,
    C_SkCanvas_isClipEmpty,
    C_SkCanvas_discard,
    SkCanvas_PointMode,
    SkImage,
    SkImageFilter,
    SkPaint,
    SkRect,
    C_SkCanvas_getBaseLayerSize,
    C_SkCanvas_imageInfo,
    C_SkCanvas_newFromBitmapAndProps,
    C_SkCanvas_newFromBitmap,
    C_SkCanvas_newWidthHeightAndProps,
    C_SkCanvas_newEmpty,
    C_SkCanvas_MakeRasterDirect,
    SkCanvas,
    C_SkCanvas_delete,
    C_SkCanvas_makeSurface,
    C_SkCanvas_getGrContext,
    SkCanvas_SaveLayerRec,
    SkCanvas_SaveLayerFlagsSet,
    SkMatrix,
    SkCanvas_SrcRectConstraint,
    C_SkAutoCanvasRestore_restore
};

bitflags! {
    pub struct SaveLayerFlags: u32 {
        const InitWithPrevious = SkCanvas_SaveLayerFlagsSet::kInitWithPrevious_SaveLayerFlag as _;
    }
}

#[allow(dead_code)]
pub struct SaveLayerRec<'a> {
    // note: we _must_ store _references_ to the
    // native types here, because not all of them
    // are native transmutable, like ImageFilter or Image,
    // which are represented as ref counted pointers and
    // so we would store a reference to a pointer only.
    bounds: Option<&'a SkRect>,
    paint: Option<&'a SkPaint>,
    backdrop: Option<&'a SkImageFilter>,
    // experimental
    clip_mask: Option<&'a SkImage>,
    // experimental
    clip_matrix: Option<&'a SkMatrix>,
    flags: SaveLayerFlags
}

impl<'a> NativeTransmutable<SkCanvas_SaveLayerRec> for SaveLayerRec<'a> {}

#[test]
fn test_save_layer_rec_layout() {
    SaveLayerRec::test_layout()
}

impl<'a> Default for SaveLayerRec<'a> {
    fn default() -> Self {
        SaveLayerRec {
            bounds: None,
            paint: None,
            backdrop: None,
            clip_mask: None,
            clip_matrix: None,
            flags: SaveLayerFlags::empty()
        }
    }
}

impl<'a> SaveLayerRec<'a> {

    pub fn bounds(self, bounds: &'a Rect) -> Self {
        Self { bounds: Some(bounds.native()), ..self }
    }

    pub fn paint(self, paint: &'a Paint) -> Self {
        Self { paint: Some(paint.native()), ..self }
    }

    pub fn backdrop(self, backdrop: &'a ImageFilter) -> Self {
        Self { backdrop: Some(backdrop.native()), ..self }
    }

    pub fn clip_mask(self, clip_mask: &'a Image) -> Self {
        Self { clip_mask: Some(clip_mask.native()), ..self }
    }

    pub fn clip_matrix(self, clip_matrix: &'a Matrix) -> Self {
        Self { clip_matrix: Some(clip_matrix.native()), ..self }
    }

    pub fn flags(self, flags: SaveLayerFlags) -> Self {
        Self { flags, .. self }
    }
}

pub type CanvasPointMode = EnumHandle<SkCanvas_PointMode>;

#[allow(non_upper_case_globals)]
impl EnumHandle<SkCanvas_PointMode> {
    pub const Points: Self = Self(SkCanvas_PointMode::kPoints_PointMode);
    pub const Lines: Self = Self(SkCanvas_PointMode::kLines_PointMode);
    pub const Polygon: Self = Self(SkCanvas_PointMode::kPolygon_PointMode);
}

pub type SrcRectConstraint = EnumHandle<SkCanvas_SrcRectConstraint>;

#[allow(non_upper_case_globals)]
impl EnumHandle<SkCanvas_SrcRectConstraint> {
    pub const Strict: Self = Self(SkCanvas_SrcRectConstraint::kStrict_SrcRectConstraint);
    pub const Fast: Self = Self(SkCanvas_SrcRectConstraint::kFast_SrcRectConstraint);
}

/// Provides access to Canvas's pixels.
/// Returned by Canvas::access_top_layer_pixels()
pub struct CanvasTopLayerPixels<'a> {
    pub pixels: &'a mut [u8],
    pub info: ImageInfo,
    pub row_bytes: usize,
    pub origin: IPoint
}

/// Additional options to Canvas's clip functions.
/// use default() for Intersect / no anti alias.

#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct CanvasClipOptions {
    pub op: ClipOp,
    pub do_anti_alias: bool
}

#[test]
pub fn canvas_clip_options_defaults() {
    let cco = CanvasClipOptions::default();
    assert_eq!(ClipOp::Intersect, cco.op);
    assert_eq!(false, cco.do_anti_alias);
}

// Warning: do never access SkCanvas fields from Rust, bindgen generates a wrong layout
// as of version 0.47.3.

/// The canvas type that is returned when it is managed by another instance,
/// like Surface, for example. For these cases, the Canvas' reference that is
/// returned is bound to the lifetime of the owner.

#[repr(transparent)]
pub struct Canvas(SkCanvas);

impl NativeAccess<SkCanvas> for Canvas {
    fn native(&self) -> &SkCanvas {
        &self.0
    }

    fn native_mut(&mut self) -> &mut SkCanvas {
        &mut self.0
    }
}

/// This is the type representing a canvas that is owned and destructed
/// when it goes out of scope _and_ is bound to a the lifetime of another
/// instance. Function resolvement is done via the Deref trait.
pub struct OwnedCanvas<'lt>(*mut Canvas, PhantomData<&'lt ()>);

impl<'lt> Deref for OwnedCanvas<'lt> {
    type Target = Canvas;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<'lt> DerefMut for OwnedCanvas<'lt> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

impl<'lt> Drop for OwnedCanvas<'lt> {
    fn drop(&mut self) {
        unsafe { C_SkCanvas_delete(self.native()) }
    }
}

impl<'lt> Default for OwnedCanvas<'lt> {
    fn default() -> Self {
        let ptr = unsafe { C_SkCanvas_newEmpty() };
        Canvas::own_from_native_ptr(ptr).unwrap()
    }
}

impl Canvas {

    pub fn from_raster_direct<'pixels>(
        info: &ImageInfo,
        pixels: &'pixels mut [u8],
        row_bytes: Option<usize>,
        props: Option<&SurfaceProps>) -> Option<OwnedCanvas<'pixels>> {
        let row_bytes = row_bytes.unwrap_or(info.min_row_bytes());
        if row_bytes >= info.min_row_bytes() && pixels.len() >= info.compute_byte_size(row_bytes) {
            let ptr = unsafe {
                C_SkCanvas_MakeRasterDirect(
                    info.native(),
                    pixels.as_mut_ptr() as _,
                    row_bytes,
                    props.native_ptr_or_null())
            };
            Self::own_from_native_ptr(ptr)
        } else {
            None
        }
    }

    pub fn from_raster_direct_n32<'pixels>(
        size: ISize,
        pixels: &'pixels mut [u32 /* PMColor */],
        row_bytes: Option<usize>) -> Option<OwnedCanvas<'pixels>> {
        let info = ImageInfo::new_n32_premul(size, None);
        let pixels_ptr : *mut u8 = pixels.as_mut_ptr() as _;
        let pixels_u8 : &'pixels mut [u8] = unsafe {
            slice::from_raw_parts_mut(pixels_ptr, pixels.elements_size_of())
        };
        Self::from_raster_direct(&info, pixels_u8, row_bytes, None)
    }

    // Decided to call this variant new, because it seems to be the simplest reasonable one.
    pub fn new<'lt>(size: ISize, props: Option<&SurfaceProps>) -> Option<OwnedCanvas<'lt>> {
        if size.width >= 0 && size.height >= 0 {
            let ptr = unsafe {
                C_SkCanvas_newWidthHeightAndProps(
                    size.width, size.height, props.native_ptr_or_null())
            };
            Canvas::own_from_native_ptr(ptr)
        } else {
            None
        }
    }

    pub fn from_bitmap<'lt>(bitmap: &Bitmap, props: Option<&SurfaceProps>) -> OwnedCanvas<'lt> {
        let props_ptr = props.native_ptr_or_null();
        let ptr =
            if props_ptr.is_null() {
                unsafe {
                    C_SkCanvas_newFromBitmap(bitmap.native())
                }
            } else {
                unsafe {
                    C_SkCanvas_newFromBitmapAndProps(bitmap.native(), props_ptr)
                }
            };
        Canvas::own_from_native_ptr(ptr).unwrap()
    }

    // TODO: getMetaData()

    pub fn image_info(&self) -> ImageInfo {
        let mut ii = ImageInfo::default();
        unsafe {
            C_SkCanvas_imageInfo(self.native(), ii.native_mut())
        };
        ii
    }

    pub fn props(&self) -> Option<SurfaceProps> {
        let mut sp = SurfaceProps::default();
        unsafe {
            self.native().getProps(sp.native_mut())
        }.if_true_some(sp)
    }

    pub fn flush(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().flush();
        }
        self
    }

    pub fn base_layer_size(&self) -> ISize {
        let mut size = ISize::default();
        unsafe {
            C_SkCanvas_getBaseLayerSize(self.native(), size.native_mut())
        }
        size
    }

    // TODO: check if the lifetime requirements are met (in relation to Surface::canvas()).
    // (we might need to consume self here and prevent the drop(), and also support
    // this function on OwnedCanvas only).
    pub fn make_surface(&mut self, info: &ImageInfo, props: Option<&SurfaceProps>) -> Option<Surface> {
        Surface::from_ptr(unsafe {
            C_SkCanvas_makeSurface(
                self.native_mut(),
                info.native(),
                props.native_ptr_or_null())
        })
    }

    // TODO: test ref count consistency assuming it is not increased in the native part.
    pub fn graphics_context(&mut self) -> Option<graphics::Context> {
        graphics::Context::from_unshared_ptr(unsafe {
            C_SkCanvas_getGrContext(self.native_mut())
        })
    }

    pub fn access_top_layer_pixels(&mut self) -> Option<CanvasTopLayerPixels> {
        let mut info = ImageInfo::default();
        let mut row_bytes = 0;
        let mut origin = IPoint::default();
        let ptr = unsafe {
            self.native_mut().accessTopLayerPixels(
                info.native_mut(),
                &mut row_bytes,
                origin.native_mut())
        };
        if !ptr.is_null() {
            let size = info.compute_byte_size(row_bytes);
            let pixels = unsafe {
                slice::from_raw_parts_mut(ptr as _, size)
            };
            Some(CanvasTopLayerPixels{pixels, info, row_bytes, origin})
        } else {
            None
        }
    }

    // TODO: accessTopRasterHandle()
    // TODO: peekPixels()

    #[warn(unused)]
    pub fn read_pixels(
        &mut self,
        info: &ImageInfo,
        dst_pixels: &mut [u8], dst_row_bytes: usize,
        src_point: IPoint) -> bool {
        let required_size = info.compute_byte_size(dst_row_bytes);
        (dst_pixels.len() >= required_size) &&
        unsafe {
            self.native_mut().readPixels(
                info.native(),
                dst_pixels.as_mut_ptr() as _, dst_row_bytes,
                src_point.x, src_point.y)
        }
    }

    // TODO: read_pixels(Pixmap).

    #[warn(unused)]
    pub fn read_pixels_to_bitmap(&mut self, bitmap: &mut Bitmap, src: IPoint) -> bool {
        unsafe {
            self.native_mut().readPixels2(bitmap.native(), src.x, src.y)
        }
    }

    // TODO: that (pixels, row_bytes) pair is probably worth abstracting over.
    #[warn(unused)]
    pub fn write_pixels(&mut self, info: &ImageInfo, pixels: &[u8], row_bytes: usize, offset: IPoint) -> bool {
        let required_size = info.compute_byte_size(row_bytes);
        (pixels.len() >= required_size) && unsafe {
            self.native_mut().writePixels(
                info.native(),
                pixels.as_ptr() as _, row_bytes,
                offset.x, offset.y)
        }
    }

    #[warn(unused)]
    pub fn write_pixels_from_bitmap(&mut self, bitmap: &Bitmap, offset: IPoint) -> bool {
        unsafe {
            self.native_mut().writePixels1(bitmap.native(), offset.x, offset.y)
        }
    }

    // TODO: (usability) think about _not_ returning usize here and instead &mut Self.
    // The count can be read via save_count() at any time.
    pub fn save(&mut self) -> usize {
        unsafe {
            self.native_mut().save().try_into().unwrap()
        }
    }

    pub fn save_layer(&mut self, layer_rec: &SaveLayerRec) -> usize {
        unsafe {
            self.native_mut().saveLayer2(layer_rec.native())
        }.try_into().unwrap()
    }

    pub fn restore(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().restore()
        };
        self
    }

    pub fn save_count(&self) -> usize {
        unsafe {
            self.native().getSaveCount()
        }.try_into().unwrap()
    }

    pub fn restore_to_count(&mut self, count: usize) -> &mut Self {
        unsafe {
            self.native_mut().restoreToCount(count.try_into().unwrap())
        }
        self
    }

    pub fn translate(&mut self, d: Vector) -> &mut Self {
        unsafe {
            self.native_mut().translate(d.x, d.y)
        }
        self
    }

    pub fn scale(&mut self, sx: scalar, sy: scalar) -> &mut Self {
        unsafe {
            self.native_mut().scale(sx, sy)
        }
        self
    }

    pub fn rotate(&mut self, degrees: scalar, point: Option<Point>) -> &mut Self {
        match point {
            Some(point) => {
                unsafe { self.native_mut().rotate1(degrees, point.x, point.y) }
            },
            None => {
                unsafe { self.native_mut().rotate(degrees) }
            }
        }
        self
    }

    pub fn skew(&mut self, sx: scalar, sy: scalar) -> &mut Self {
        unsafe {
            self.native_mut().skew(sx, sy)
        }
        self
    }

    pub fn concat(&mut self, matrix: &Matrix) -> &mut Self {
        unsafe {
            self.native_mut().concat(matrix.native())
        }
        self
    }

    pub fn set_matrix(&mut self, matrix: &Matrix) -> &mut Self {
        unsafe {
            self.native_mut().setMatrix(matrix.native())
        }
        self
    }

    pub fn reset_matrix(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().resetMatrix()
        }
        self
    }

    pub fn clip_rect(&mut self, rect: &Rect, options: CanvasClipOptions) -> &mut Self {
        unsafe {
            self.native_mut().clipRect(
                rect.native(),
                options.op.into_native(), options.do_anti_alias)
        }
        self
    }

    pub fn clip_rrect(&mut self, rrect: &RRect, options: CanvasClipOptions) -> &mut Self {
        unsafe {
            self.native_mut().clipRRect(
                rrect.native(),
                options.op.into_native(), options.do_anti_alias)
        }
        self
    }

    pub fn clip_path(&mut self, path: &Path, options: CanvasClipOptions) -> &mut Self {
        unsafe {
            self.native_mut().clipPath(
                path.native(),
                options.op.into_native(), options.do_anti_alias)
        }
        self
    }

    pub fn clip_region(&mut self, device_rgn: &Region, op: ClipOp) -> &mut Self {
        unsafe {
            self.native_mut().clipRegion(device_rgn.native(), op.into_native())
        }
        self
    }

    // quickReject is implemented as a trait.
    // TODO: think about removing that trait and implement
    // quick_reject_rect and quick_reject_path here and for impl Region.

    pub fn local_clip_bounds(&self) -> Option<Rect> {
        let r = Rect::from_native(unsafe {
            // pointer versions do not link.
            self.native().getLocalClipBounds()
        });
        r.is_empty().if_false_some(r)
    }

    pub fn device_clip_bounds(&self) -> Option<IRect> {
        let r = IRect::from_native(unsafe {
            // pointer versions do not link.
            self.native().getDeviceClipBounds()
        });
        r.is_empty().if_false_some(r)
    }

    pub fn draw_color(&mut self, color: Color, mode: BlendMode) -> &mut Self {
        unsafe {
            self.native_mut().drawColor(color.into_native(), mode.into_native())
        }
        self
    }

    pub fn clear(&mut self, color: Color) -> &mut Self {
        unsafe {
            self.native_mut().clear(color.into_native())
        }
        self
    }

    pub fn discard(&mut self) -> &mut Self {
        unsafe {
            // does not link:
            // self.native_mut().discard()
            C_SkCanvas_discard(self.native_mut())
        }
        self
    }

    pub fn draw_paint(&mut self, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawPaint(paint.native())
        }
        self
    }

    pub fn draw_points(&mut self, mode: CanvasPointMode, pts: &[Point], paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawPoints(
                mode.into_native(), pts.len(), pts.native().as_ptr(), paint.native())
        }
        self
    }

    pub fn draw_point(&mut self, p: Point, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawPoint(p.x, p.y, paint.native())
        }
        self
    }

    pub fn draw_line(&mut self, p1: Point, p2: Point, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawLine(p1.x, p1.y, p2.x, p2.y, paint.native())
        }
        self
    }

    pub fn draw_rect(&mut self, rect: &Rect, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawRect(rect.native(), paint.native())
        }
        self
    }

    pub fn draw_irect(&mut self, rect: &IRect, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawIRect(rect.native(), paint.native())
        }
        self
    }

    pub fn draw_region(&mut self, region: &Region, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawRegion(region.native(), paint.native())
        }
        self
    }

    pub fn draw_oval(&mut self, oval: &Rect, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawOval(oval.native(), paint.native())
        }
        self
    }

    pub fn draw_rrect(&mut self, rrect: &RRect, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawRRect(rrect.native(), paint.native())
        }
        self
    }

    pub fn draw_drrect(&mut self, outer: &RRect, inner: &RRect, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawDRRect(outer.native(), inner.native(), paint.native())
        }
        self
    }

    pub fn draw_circle(&mut self, center: Point, radius: scalar, paint: &Paint) -> &mut Self {
        unsafe {
            // does not link:
            // self.native_mut().drawCircle1(center.into_native(), radius, paint.native())
            self.native_mut().drawCircle(center.x, center.y, radius, paint.native())
        }
        self
    }

    pub fn draw_arc(&mut self, oval: &Rect, start_angle: scalar, sweep_angle: scalar, use_center: bool, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawArc(
                oval.native(),
                start_angle, sweep_angle,
                use_center, paint.native())
        }
        self
    }

    pub fn draw_round_rect(&mut self, rect: &Rect, rx: scalar, ry: scalar, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawRoundRect(rect.native(), rx, ry, paint.native())
        }
        self
    }

    pub fn draw_path(&mut self, path: &Path, paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawPath(path.native(), paint.native())
        }
        self
    }

    pub fn draw_image(&mut self, image: &Image, left_top: Point, paint: Option<&Paint>) -> &mut Self {
        unsafe {
            self.native_mut().drawImage(
                image.native(), left_top.x, left_top.y,
                paint.native_ptr_or_null())
        }
        self
    }

    pub fn draw_image_rect(
        &mut self,
        image: &Image,
        src: Option<(&Rect, SrcRectConstraint)>,
        dst: &Rect,
        paint: &Paint) -> &mut Self {
        match src {
            Some((src, constraint)) => unsafe {
                self.native_mut().drawImageRect(
                    image.native(),
                    src.native(), dst.native(),
                    paint.native(), constraint.into_native())
            },
            None => unsafe {
                self.native_mut().drawImageRect2(
                    image.native(),
                    dst.native(),
                    paint.native())
            }
        }
        self
    }

    pub fn draw_image_nine(
        &mut self, image: &Image, center: &IRect,
        dst: &Rect, paint: Option<&Paint>) -> &mut Self {
        unsafe {
            self.native_mut().drawImageNine(
                image.native(), center.native(),
                dst.native(), paint.native_ptr_or_null())
        }
        self
    }

    pub fn draw_bitmap(&mut self, bitmap: &Bitmap, left_top: Point, paint: Option<&Paint>) -> &mut Self {
        unsafe {
            self.native_mut().drawBitmap(
                bitmap.native(), left_top.x, left_top.y,
                paint.native_ptr_or_null())
        }
        self
    }

    pub fn draw_bitmap_rect(
        &mut self,
        bitmap: &Bitmap,
        src: Option<&Rect>,
        dst: &Rect,
        paint: &Paint,
        constraint: SrcRectConstraint) -> &mut Self {
        match src {
            Some(src) => unsafe {
                self.native_mut().drawBitmapRect(
                    bitmap.native(),
                    src.native(), dst.native(),
                    paint.native(), constraint.into_native())
            },
            None => unsafe {
                self.native_mut().drawBitmapRect2(
                    bitmap.native(),
                    dst.native(),
                    paint.native(),
                    constraint.into_native())
            }
        }
        self
    }

    pub fn draw_bitmap_nine(
        &mut self, bitmap: &Bitmap, center: &IRect,
        dst: &Rect, paint: Option<&Paint>) -> &mut Self {
        unsafe {
            self.native_mut().drawBitmapNine(
                bitmap.native(), center.native(),
                dst.native(), paint.native_ptr_or_null())
        }
        self
    }

    // TODO: Lattice, drawBitmapLattice, drawImageLattice

    // TODO: drawSimpleText

    // rust specific, based on drawSimpleText with fixed UTF8 encoding,
    // implementation is similar to Font's *_str methods.
    pub fn draw_str(&mut self, str: &str, origin: Point, font: &Font, paint: &Paint) -> &mut Self {
        let bytes = str.as_bytes();
        unsafe {
            self.native_mut().drawSimpleText(
                bytes.as_ptr() as _, bytes.len(), TextEncoding::UTF8.into_native(),
                origin.x, origin.y, font.native(), paint.native())
        }
        self
    }

    // TODO: drawTextBlob

    pub fn draw_picture(&mut self, picture: &Picture, matrix: Option<&Matrix>, paint: Option<&Paint>) -> &mut Self {
        unsafe {
            self.native_mut().drawPicture2(
                picture.native(),
                matrix.native_ptr_or_null(),
                paint.native_ptr_or_null())
        }
        self
    }

    pub fn draw_vertices(
        &mut self,
        vertices: &Vertices, bones: Option<&[VerticesBone]>, mode: BlendMode, paint: &Paint) -> &mut Self {
        match bones {
            Some(bones) => unsafe {
                self.native_mut().drawVertices2(
                    vertices.native(),
                    bones.native().as_ptr(),
                    bones.len().try_into().unwrap(),
                    mode.into_native(),
                    paint.native())
            },
            None => unsafe {
                self.native_mut().drawVertices(
                    vertices.native(),
                    mode.into_native(),
                    paint.native())
            }
        }
        self
    }

    pub fn draw_patch(
        &mut self,
        cubics: &[Point;12],
        colors: &[Color;4],
        tex_coords: &[Point;4],
        mode: BlendMode,
        paint: &Paint) -> &mut Self {
        unsafe {
            self.native_mut().drawPatch(
                cubics.native().as_ptr(),
                colors.native().as_ptr(),
                tex_coords.native().as_ptr(),
                mode.into_native(),
                paint.native())
        }
        self
    }

    // TODO: drawAtlas
    // TODO: drawDrawable

    // TODO: why is Data mutable here?
    pub fn draw_annotation(&mut self, rect: &Rect, key: &str, value: &mut Data) -> &mut Self {
        let key = CString::new(key).unwrap();
        unsafe {
            self.native_mut().drawAnnotation(
                rect.native(),
                key.as_ptr(),
                value.native_mut() )
        }
        self
    }

    pub fn is_clip_empty(&self) -> bool {
        unsafe {
            C_SkCanvas_isClipEmpty(self.native())
        }
    }

    pub fn is_clip_rect(&self) -> bool {
        unsafe {
            C_SkCanvas_isClipEmpty(self.native())
        }
    }

    pub fn total_matrix(&self) -> &Matrix {
        // TODO: make this official, transmutation of a Matrix is not actually supported.
        let matrix = unsafe {
            &*self.native().getTotalMatrix()
        };
        unsafe { mem::transmute::<&SkMatrix, &Matrix>(matrix) }
    }

    //
    // internal helper
    //

    pub(crate) fn own_from_native_ptr<'lt>(native: *mut SkCanvas) -> Option<OwnedCanvas<'lt>> {
        if !native.is_null() {
            Some(OwnedCanvas::<'lt>(
                Self::borrow_from_native(unsafe {
                    &mut *native
                }), PhantomData))
        } else {
            None
        }
    }

    pub(crate) fn borrow_from_native(native: &mut SkCanvas) -> &mut Self {
        unsafe {
            mem::transmute::<&mut SkCanvas, &mut Self>(native)
        }
    }
}

impl QuickReject<Rect> for Canvas {
    fn quick_reject(&self, other: &Rect) -> bool {
        unsafe {
            self.native().quickReject(other.native())
        }
    }
}

impl QuickReject<Path> for Canvas {
    fn quick_reject(&self, other: &Path) -> bool {
        unsafe {
            self.native().quickReject1(other.native())
        }
    }
}

pub struct AutoCanvasRestore<'a>(SkAutoCanvasRestore, PhantomData<&'a ()>);

impl<'a> NativeAccess<SkAutoCanvasRestore> for AutoCanvasRestore<'a> {
    fn native(&self) -> &SkAutoCanvasRestore {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkAutoCanvasRestore {
        &mut self.0
    }
}

impl<'a> Drop for AutoCanvasRestore<'a> {
    fn drop(&mut self) {
        unsafe {
            C_SkAutoCanvasRestore_destruct(&self.0)
        }
    }
}

impl<'a> AutoCanvasRestore<'a> {
    // TODO: test, scary looking lifetime requirements.
    pub fn guard(canvas: &mut Canvas, do_save: bool) -> AutoCanvasRestore<'_> {
        AutoCanvasRestore(unsafe {
            SkAutoCanvasRestore::new(canvas.native_mut(), do_save)
        }, PhantomData)
    }

    pub fn restore(&mut self) {
        unsafe {
            // does not link:
            // self.native_mut().restore()
            C_SkAutoCanvasRestore_restore(self.native_mut())
        }
    }
}

#[test]
fn test_raster_direct_creation_and_clear_in_memory() {
    let info = ImageInfo::new((2, 2).into(), ColorType::RGBA8888, AlphaType::Unpremul, None);
    assert_eq!(8, info.min_row_bytes());
    let mut bytes : [u8; 8*2] = Default::default();
    {
        let mut canvas = Canvas::from_raster_direct(&info, bytes.as_mut(), None, None).unwrap();
        canvas.clear(Color::RED);
    }

    assert_eq!(0xff, bytes[0]);
    assert_eq!(0x00, bytes[1]);
    assert_eq!(0x00, bytes[2]);
    assert_eq!(0xff, bytes[3]);
}

#[test]
fn test_raster_direct_n32_creation_and_clear_in_memory() {
    let mut pixels : [u32; 4] = Default::default();
    {
        let mut canvas = Canvas::from_raster_direct_n32(
            (2,2).into(),
            pixels.as_mut(),
            None).unwrap();
        canvas.clear(Color::RED);
    }

    assert_eq!(0xffff0000, pixels[0]);
}

#[test]
fn test_empty_canvas_creation() {
    let canvas = OwnedCanvas::default();
    drop(canvas)
}

#[test]
fn test_save_layer_rec_lifetimes() {
    let rect = Rect::default();
    {
        let matrix = Matrix::default();

        let rec = SaveLayerRec::default()
            .clip_matrix(&matrix)
            .bounds(&rect);
    }
}

#[test]
fn test_total_matrix_transmutation() {
    let mut c = Canvas::new((2, 2).into(), None).unwrap();
    let matrix_ref = c.total_matrix();
    assert!(Matrix::default() == *matrix_ref);
    c.rotate(0.1, None);
    let matrix_ref = c.total_matrix();
    assert!(Matrix::default() != *matrix_ref);
}

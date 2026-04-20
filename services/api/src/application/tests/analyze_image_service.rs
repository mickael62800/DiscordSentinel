use super::*;

#[test]
fn test_preprocess_invalid_bytes_returns_error() {
    let result = preprocess_image(&[0, 1, 2, 3]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Image invalide"));
}

#[test]
fn test_preprocess_valid_png() {
    let mut buf = Vec::new();
    {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(2, 2, |_, _| Rgb([128, 64, 200]));
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tensor = preprocess_image(&buf).unwrap();
    assert_eq!(tensor.shape(), &[1, 3, 224, 224]);
}

#[test]
fn test_preprocess_normalization_range() {
    let mut buf = Vec::new();
    {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(1, 1, |_, _| Rgb([255, 255, 255]));
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tensor = preprocess_image(&buf).unwrap();
    let val_r = tensor[[0, 0, 0, 0]];
    assert!((val_r - 2.249).abs() < 0.01);
}

#[test]
fn test_preprocess_black_pixel_normalization() {
    let mut buf = Vec::new();
    {
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(1, 1, |_, _| Rgb([0, 0, 0]));
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    }
    let tensor = preprocess_image(&buf).unwrap();
    let val_r = tensor[[0, 0, 0, 0]];
    assert!((val_r - (-2.118)).abs() < 0.01);
}

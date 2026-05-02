import jsQR from 'jsqr';

/**
 * Decode first QR code in an image (paste / file). Returns raw string inside the QR.
 */
export async function decodeQrFromImageBlob(blob: Blob): Promise<string | null> {
  const bmp = await createImageBitmap(blob);
  const canvas = document.createElement('canvas');
  const maxDim = 1600;
  let w = bmp.width;
  let h = bmp.height;
  if (w > maxDim || h > maxDim) {
    const scale = maxDim / Math.max(w, h);
    w = Math.floor(w * scale);
    h = Math.floor(h * scale);
  }
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    return null;
  }
  ctx.drawImage(bmp, 0, 0, w, h);
  bmp.close?.();
  const imageData = ctx.getImageData(0, 0, w, h);
  const code = jsQR(imageData.data, imageData.width, imageData.height, {
    inversionAttempts: 'attemptBoth',
  });
  return code?.data ?? null;
}

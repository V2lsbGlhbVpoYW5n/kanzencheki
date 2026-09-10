import { demoPhotos, type Photo } from './demo';

// Shared for this app session: changing routes must not discard imported files.
// Blob URLs remain valid until the document closes; this is not disk persistence.
export const librarySession = $state<{ photos: Photo[] }>({
  photos: structuredClone(demoPhotos).map(photo => ({
    ...photo,
    images: [{ id: `${photo.id}-image`, src: photo.src, name: '原始影像' }]
  }))
});

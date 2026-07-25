import { useState } from 'react';
import { uploadImage } from '../utils/api';

interface ImageUploadProps {
  onUploadComplete: (url: string) => void;
}

export function ImageUpload({ onUploadComplete }: ImageUploadProps) {
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    // Validate file type
    if (!file.type.startsWith('image/')) {
      setError('Please select an image file');
      return;
    }

    // Validate file size (5MB max)
    if (file.size > 5 * 1024 * 1024) {
      setError('File size must be less than 5MB');
      return;
    }

    setUploading(true);
    setError(null);

    try {
      const response = await uploadImage(file);
      onUploadComplete(response.url);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed');
    } finally {
      setUploading(false);
    }
  };

  return (
    <div className="image-upload">
      <label htmlFor="cover-image" className="form-label">
        Cover Image
      </label>
      <input
        id="cover-image"
        type="file"
        accept="image/*"
        onChange={handleFileChange}
        disabled={uploading}
        className="file-input"
        aria-describedby={error ? 'upload-error' : undefined}
      />
      {uploading && <p className="upload-status">Uploading...</p>}
      {error && (
        <p id="upload-error" className="error-message" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

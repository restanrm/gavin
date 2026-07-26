import { useEffect, useRef, useState } from 'react';
import { createVinylFromCandidate, importCoverImage } from '../utils/api';
import type { AlbumCandidate, CoverImportResponse } from '../types';

interface CoverImportProps {
  onSuccess: () => void;
}

export function CoverImport({ onSuccess }: CoverImportProps) {
  const [file, setFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [selectingId, setSelectingId] = useState<string | null>(null);
  const [result, setResult] = useState<CoverImportResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const cameraInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    return () => {
      if (previewUrl) {
        URL.revokeObjectURL(previewUrl);
      }
    };
  }, [previewUrl]);

  const handleSelectedFile = (selectedFile: File | null) => {
    if (previewUrl) {
      URL.revokeObjectURL(previewUrl);
    }

    setFile(selectedFile);
    setPreviewUrl(selectedFile ? URL.createObjectURL(selectedFile) : null);
    setResult(null);
    setError(null);
  };

  const handleFileInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    handleSelectedFile(event.target.files?.[0] ?? null);
    event.target.value = '';
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();

    if (!file) {
      setError('Please select an album-cover image');
      return;
    }

    if (!file.type.startsWith('image/')) {
      setError('Please select an image file');
      return;
    }

    if (file.size > 10 * 1024 * 1024) {
      setError('File size must be less than 10MB');
      return;
    }

    setSubmitting(true);
    setError(null);
    setResult(null);

    try {
      const response = await importCoverImage(file);
      setResult(response);
      if (response.vinyl) {
        handleSelectedFile(null);
        onSuccess();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Cover import failed');
    } finally {
      setSubmitting(false);
    }
  };

  const handleCandidateImport = async (candidate: AlbumCandidate) => {
    setSelectingId(candidate.id);
    setError(null);

    try {
      await createVinylFromCandidate(candidate);
      handleSelectedFile(null);
      setResult(null);
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to import selected album');
    } finally {
      setSelectingId(null);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="cover-import">
      <h3>Import Album from Cover Photo</h3>
      <p className="help-text">
        Upload a photo of the album cover. Gavin uses the configured vision provider to identify the album, then stores MusicBrainz metadata and the official Cover Art Archive image instead of your photo.
      </p>

      {error && (
        <div className="error-message" role="alert">
          {error}
        </div>
      )}

      <div className="form-group">
        <span className="form-label">Cover photo to identify</span>
        <div className="cover-import-picker">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={() => fileInputRef.current?.click()}
            disabled={submitting}
          >
            Choose image
          </button>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={() => cameraInputRef.current?.click()}
            disabled={submitting}
          >
            Take photo
          </button>
        </div>
        <input
          ref={fileInputRef}
          id="cover-import-image"
          type="file"
          accept="image/*"
          onChange={handleFileInputChange}
          disabled={submitting}
          className="cover-import-file-input"
          aria-hidden="true"
          tabIndex={-1}
        />
        <input
          ref={cameraInputRef}
          type="file"
          accept="image/*"
          capture="environment"
          onChange={handleFileInputChange}
          disabled={submitting}
          className="cover-import-file-input"
          aria-hidden="true"
          tabIndex={-1}
        />
        <p className="help-text cover-import-picker-help">
          On phones and tablets, “Take photo” opens the camera when supported.
        </p>
        {file && <p className="upload-status">Selected: {file.name}</p>}
      </div>

      <button
        type="submit"
        disabled={submitting || !file}
        className="btn btn-primary"
      >
        {submitting ? 'Finding album...' : 'Find Matching Jackets'}
      </button>

      {result?.status === 'complete' && result.vinyl && (
        <p className="upload-status" role="status">
          Imported {result.vinyl.artist} — {result.vinyl.title}
        </p>
      )}

      {result?.status === 'not_found' && (
        <div className="cover-import-result" role="status">
          <p>No album match found from this image.</p>
          {result.detected_terms.length > 0 && (
            <details>
              <summary>Detected album search terms</summary>
              <pre>{result.detected_terms.join('\n')}</pre>
            </details>
          )}
        </div>
      )}

      {result?.status === 'error' && (
        <div className="error-message" role="alert">
          {result.error ?? 'Could not analyze this image'}
        </div>
      )}

      {result?.status === 'needs_choice' && (
        <div className="cover-import-result">
          <p>Choose the jacket that matches your uploaded cover before importing:</p>
          <div className="cover-choice-layout">
            {previewUrl && (
              <div className="uploaded-cover-preview">
                <strong>Uploaded cover</strong>
                <img src={previewUrl} alt="Uploaded album cover" />
              </div>
            )}
            <ul className="cover-candidate-list">
              {result.candidates.map((candidate) => (
                <li key={candidate.id} className="cover-candidate">
                  <button
                    type="button"
                    className="cover-candidate-button"
                    onClick={() => handleCandidateImport(candidate)}
                    disabled={selectingId !== null}
                  >
                    {candidate.cover_image_url && (
                      <img
                        src={candidate.cover_image_url}
                        alt={`${candidate.artist} — ${candidate.title} cover`}
                        loading="lazy"
                      />
                    )}
                    <span className="cover-candidate-info">
                      <strong>{candidate.title}</strong>
                      <span>{candidate.artist}</span>
                      {candidate.release_year && <span>{candidate.release_year}</span>}
                      {candidate.genre && <span>{candidate.genre}</span>}
                      {candidate.disambiguation && <small>{candidate.disambiguation}</small>}
                      <span className="cover-candidate-action">
                        {selectingId === candidate.id ? 'Importing...' : 'Click this jacket to import'}
                      </span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </form>
  );
}

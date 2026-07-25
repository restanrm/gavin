import type { Vinyl } from '../types';

interface VinylCardProps {
  vinyl: Vinyl;
  isAdmin?: boolean;
  onDelete?: (id: string) => void;
}

interface AlbumCandidate {
  id: string;
  title: string;
  artist: string;
  release_year?: number;
  disambiguation?: string;
  source_url: string;
}

function parseCandidates(value?: string): AlbumCandidate[] {
  if (!value) {
    return [];
  }

  try {
    return JSON.parse(value) as AlbumCandidate[];
  } catch {
    return [];
  }
}

function metadataLabel(status: Vinyl['metadata_status']): string {
  switch (status) {
    case 'pending':
      return 'Metadata pending';
    case 'needs_choice':
      return 'Metadata choice required';
    case 'not_found':
      return 'Metadata not found';
    case 'error':
      return 'Metadata error';
    case 'disabled':
      return 'Metadata disabled';
    case 'complete':
    default:
      return 'Metadata complete';
  }
}

export function VinylCard({ vinyl, isAdmin = false, onDelete }: VinylCardProps) {
  const candidates = parseCandidates(vinyl.metadata_candidates);

  const handleDelete = () => {
    if (window.confirm(`Delete "${vinyl.title}" by ${vinyl.artist}?`)) {
      onDelete?.(vinyl.id);
    }
  };

  return (
    <article className="vinyl-card">
      <div className="vinyl-cover">
        {vinyl.cover_image_url ? (
          <img
            src={vinyl.cover_image_url}
            alt={`${vinyl.title} album cover`}
            loading="lazy"
          />
        ) : (
          <div className="vinyl-placeholder" aria-hidden="true">
            <svg width="80" height="80" viewBox="0 0 80 80" fill="none">
              <circle cx="40" cy="40" r="35" stroke="currentColor" strokeWidth="2" />
              <circle cx="40" cy="40" r="8" fill="currentColor" />
              <circle cx="40" cy="40" r="20" stroke="currentColor" strokeWidth="1" opacity="0.3" />
            </svg>
          </div>
        )}
      </div>
      <div className="vinyl-info">
        <h3 className="vinyl-title">{vinyl.title}</h3>
        <p className="vinyl-artist">{vinyl.artist}</p>
        {vinyl.release_year && (
          <p className="vinyl-year" aria-label={`Released in ${vinyl.release_year}`}>
            {vinyl.release_year}
          </p>
        )}
        {vinyl.notes && (
          <p className="vinyl-notes">{vinyl.notes}</p>
        )}
        {isAdmin && vinyl.metadata_status && vinyl.metadata_status !== 'complete' && (
          <div className={`metadata-badge metadata-${vinyl.metadata_status}`}>
            {metadataLabel(vinyl.metadata_status)}
          </div>
        )}
        {isAdmin && vinyl.metadata_status === 'needs_choice' && candidates.length > 0 && (
          <details className="metadata-candidates">
            <summary>Review possible album matches</summary>
            <ul>
              {candidates.map((candidate) => (
                <li key={candidate.id}>
                  <a href={candidate.source_url} target="_blank" rel="noreferrer">
                    {candidate.artist} — {candidate.title}
                    {candidate.release_year ? ` (${candidate.release_year})` : ''}
                  </a>
                  {candidate.disambiguation ? ` — ${candidate.disambiguation}` : ''}
                </li>
              ))}
            </ul>
          </details>
        )}
        {isAdmin && vinyl.metadata_error && vinyl.metadata_status !== 'needs_choice' && (
          <p className="metadata-error">{vinyl.metadata_error}</p>
        )}
      </div>
      {isAdmin && onDelete && (
        <button
          onClick={handleDelete}
          className="btn btn-danger btn-sm delete-btn"
          aria-label={`Delete ${vinyl.title}`}
        >
          Delete
        </button>
      )}
    </article>
  );
}

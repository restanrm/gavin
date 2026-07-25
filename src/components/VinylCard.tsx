import type { Vinyl } from '../types';

interface VinylCardProps {
  vinyl: Vinyl;
  isAdmin?: boolean;
  onDelete?: (id: string) => void;
}

export function VinylCard({ vinyl, isAdmin = false, onDelete }: VinylCardProps) {
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

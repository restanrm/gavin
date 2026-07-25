import { VinylCard } from './VinylCard';
import type { Vinyl } from '../types';
import { deleteVinyl } from '../utils/api';

interface VinylCatalogProps {
  vinyls: Vinyl[];
  loading: boolean;
  error: string | null;
  isAdmin: boolean;
  onVinylsUpdate: () => void;
}

export function VinylCatalog({ vinyls, loading, error, isAdmin, onVinylsUpdate }: VinylCatalogProps) {
  const handleDelete = async (id: string) => {
    try {
      await deleteVinyl(id);
      onVinylsUpdate();
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete vinyl');
    }
  };

  if (loading) {
    return (
      <div className="catalog-status" role="status" aria-live="polite">
        <p>Loading vinyls...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="catalog-status error" role="alert">
        <p>Error: {error}</p>
      </div>
    );
  }

  if (vinyls.length === 0) {
    return (
      <div className="catalog-status empty">
        <p>No vinyls found. {isAdmin && 'Add some using the form above!'}</p>
      </div>
    );
  }

  return (
    <div className="vinyl-catalog">
      <div className="vinyl-grid" role="list">
        {vinyls.map((vinyl) => (
          <div key={vinyl.id} role="listitem">
            <VinylCard
              vinyl={vinyl}
              isAdmin={isAdmin}
              onDelete={isAdmin ? handleDelete : undefined}
              onUpdate={isAdmin ? onVinylsUpdate : undefined}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

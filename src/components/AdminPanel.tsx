import { VinylForm } from './VinylForm';
import { BulkImport } from './BulkImport';

interface AdminPanelProps {
  onVinylsUpdate: () => void;
}

export function AdminPanel({ onVinylsUpdate }: AdminPanelProps) {
  return (
    <div className="admin-panel">
      <h2>Admin Controls</h2>

      <div className="admin-sections">
        <section className="admin-section">
          <VinylForm onSuccess={onVinylsUpdate} />
        </section>

        <section className="admin-section">
          <BulkImport onSuccess={onVinylsUpdate} />
        </section>
      </div>
    </div>
  );
}

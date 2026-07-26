import { useEffect, useState } from 'react';
import { VinylForm } from './VinylForm';
import { BulkImport } from './BulkImport';
import { CoverImport } from './CoverImport';
import { ArtistAlbumImport } from './ArtistAlbumImport';
import { cleanupOrphanedImages, refreshMissingMetadata } from '../utils/api';

interface AdminPanelProps {
  onVinylsUpdate: () => void;
}

const ADMIN_PANEL_COLLAPSED_KEY = 'gavin-admin-panel-collapsed';

type MaintenanceAction = 'metadata' | 'cleanup';

export function AdminPanel({ onVinylsUpdate }: AdminPanelProps) {
  const [collapsed, setCollapsed] = useState(() => (
    window.localStorage.getItem(ADMIN_PANEL_COLLAPSED_KEY) === 'true'
  ));
  const [runningAction, setRunningAction] = useState<MaintenanceAction | null>(null);
  const [maintenanceMessage, setMaintenanceMessage] = useState<string | null>(null);
  const [maintenanceError, setMaintenanceError] = useState<string | null>(null);

  useEffect(() => {
    window.localStorage.setItem(ADMIN_PANEL_COLLAPSED_KEY, String(collapsed));
  }, [collapsed]);

  const runMetadataRefresh = async () => {
    setRunningAction('metadata');
    setMaintenanceMessage(null);
    setMaintenanceError(null);

    try {
      const result = await refreshMissingMetadata();
      setMaintenanceMessage(`Metadata refresh checked ${result.checked} album${result.checked === 1 ? '' : 's'}.`);
      onVinylsUpdate();
    } catch (err) {
      setMaintenanceError(err instanceof Error ? err.message : 'Failed to refresh missing metadata');
    } finally {
      setRunningAction(null);
    }
  };

  const runOrphanCleanup = async () => {
    setRunningAction('cleanup');
    setMaintenanceMessage(null);
    setMaintenanceError(null);

    try {
      const result = await cleanupOrphanedImages();
      const errorSuffix = result.errors.length > 0 ? ` ${result.errors.length} file${result.errors.length === 1 ? '' : 's'} could not be removed.` : '';
      setMaintenanceMessage(`Deleted ${result.deleted} orphaned image${result.deleted === 1 ? '' : 's'}; kept ${result.kept} referenced image${result.kept === 1 ? '' : 's'}.${errorSuffix}`);
    } catch (err) {
      setMaintenanceError(err instanceof Error ? err.message : 'Failed to clean orphaned images');
    } finally {
      setRunningAction(null);
    }
  };

  return (
    <div className={`admin-panel ${collapsed ? 'admin-panel-collapsed' : ''}`}>
      <div className="admin-panel-header">
        <h2>Admin Controls</h2>
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          onClick={() => setCollapsed((value) => !value)}
          aria-expanded={!collapsed}
          aria-controls="admin-panel-content"
        >
          {collapsed ? 'Show controls' : 'Hide controls'}
        </button>
      </div>

      {!collapsed && (
        <div id="admin-panel-content">
          <section className="admin-section admin-maintenance-section" aria-labelledby="admin-maintenance-heading">
            <h3 id="admin-maintenance-heading">Maintenance</h3>
            <p className="help-text">
              Run collection-wide maintenance after edits or imports.
            </p>
            <div className="admin-maintenance-actions">
              <button
                type="button"
                className="btn btn-secondary"
                onClick={runMetadataRefresh}
                disabled={runningAction !== null}
              >
                {runningAction === 'metadata' ? 'Refreshing metadata…' : 'Refresh missing metadata'}
              </button>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={runOrphanCleanup}
                disabled={runningAction !== null}
              >
                {runningAction === 'cleanup' ? 'Cleaning images…' : 'Clean orphaned images'}
              </button>
            </div>
            {maintenanceMessage && <p className="admin-maintenance-status" role="status">{maintenanceMessage}</p>}
            {maintenanceError && <p className="metadata-error" role="alert">{maintenanceError}</p>}
          </section>

          <div className="admin-sections">
            <section className="admin-section admin-section-featured">
              <CoverImport onSuccess={onVinylsUpdate} />
            </section>

            <section className="admin-section">
              <ArtistAlbumImport onSuccess={onVinylsUpdate} />
            </section>

            <section className="admin-section">
              <VinylForm onSuccess={onVinylsUpdate} />
            </section>

            <section className="admin-section">
              <BulkImport onSuccess={onVinylsUpdate} />
            </section>
          </div>
        </div>
      )}
    </div>
  );
}

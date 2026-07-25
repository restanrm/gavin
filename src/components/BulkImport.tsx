import { useState } from 'react';
import { bulkImportVinyls } from '../utils/api';
import { parseCSV } from '../utils/csv';

interface BulkImportProps {
  onSuccess: () => void;
}

export function BulkImport({ onSuccess }: BulkImportProps) {
  const [csvText, setCsvText] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [validationErrors, setValidationErrors] = useState<string[]>([]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!csvText.trim()) {
      setError('Please enter CSV data');
      return;
    }

    const { items, errors } = parseCSV(csvText);

    if (errors.length > 0) {
      setValidationErrors(errors);
      setError('Please fix validation errors');
      return;
    }

    if (items.length === 0) {
      setError('No valid items to import');
      return;
    }

    setSubmitting(true);
    setError(null);
    setValidationErrors([]);

    try {
      await bulkImportVinyls(items);
      setCsvText('');
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Bulk import failed');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="bulk-import">
      <h3>Bulk Import (CSV)</h3>

      <p className="help-text">
        Format: <code>artist,title,year,notes,cover_url</code> (one per line)
        <br />
        Artist and title are required. Year, notes, and cover URL are optional. Album metadata is fetched online during import; rows with multiple matches are marked for review.
      </p>

      <details className="example-details">
        <summary>Show example</summary>
        <pre className="example-csv">
{`The Beatles,Abbey Road,1969,Final studio album,https://example.com/abbey.jpg
Pink Floyd,The Dark Side of the Moon,1973
Miles Davis,Kind of Blue,1959,Essential jazz album`}
        </pre>
      </details>

      {error && (
        <div className="error-message" role="alert">
          {error}
        </div>
      )}

      {validationErrors.length > 0 && (
        <div className="validation-errors" role="alert">
          <strong>Validation Errors:</strong>
          <ul>
            {validationErrors.map((err, index) => (
              <li key={index}>{err}</li>
            ))}
          </ul>
        </div>
      )}

      <div className="form-group">
        <label htmlFor="csv-input" className="form-label">
          CSV Data
        </label>
        <textarea
          id="csv-input"
          value={csvText}
          onChange={(e) => {
            setCsvText(e.target.value);
            setValidationErrors([]);
            setError(null);
          }}
          disabled={submitting}
          className="form-textarea csv-textarea"
          rows={10}
          placeholder="The Beatles,Abbey Road,1969"
          aria-describedby={validationErrors.length > 0 ? 'validation-errors' : undefined}
        />
      </div>

      <button
        type="submit"
        disabled={submitting || !csvText.trim()}
        className="btn btn-primary"
      >
        {submitting ? 'Importing...' : 'Import Vinyls'}
      </button>
    </form>
  );
}

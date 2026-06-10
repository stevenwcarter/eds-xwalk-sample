// Contrived demo block: renders one row per field as `fieldName: value`, so each
// top-level component-model field type can be verified in the editor. Multi-value
// fields (multiselect / checkbox-group / aem-tag) are already comma-joined by the
// server, giving `fieldName: value, value, value`.

// Fallback field order, matching the `fieldsdemo` model. Used on the delivery port
// where `data-aue-prop` is absent; in the editor the live prop name is preferred.
const FIELD_NAMES = [
  'textField',
  'textareaField',
  'richtextField',
  'numberField',
  'booleanField',
  'selectField',
  'multiselectField',
  'checkboxGroupField',
  'radioGroupField',
  'dateField',
  'timeField',
  'dateTimeField',
  'referenceField',
  'aemContentField',
  'aemTagField',
  'contentFragmentField',
  'experienceFragmentField',
];

export default function decorate(block) {
  [...block.children].forEach((row, i) => {
    const cell = row.firstElementChild;
    if (!cell) return;
    const name = cell.dataset.aueProp || FIELD_NAMES[i] || `field${i}`;
    const label = document.createElement('span');
    label.className = 'fieldsdemo-label';
    label.textContent = `${name}:`;
    // Insert the label before the value cell rather than replacing the cell, so the
    // editor's per-field instrumentation on the value div stays intact.
    row.prepend(label);
    row.classList.add('fieldsdemo-row');
  });
}

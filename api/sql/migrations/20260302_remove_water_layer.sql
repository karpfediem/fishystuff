-- Remove deprecated water overlay layer.
-- Existing water assets are non-authoritative and should not be served/rendered.

DELETE FROM event_zone_assignment
WHERE layer_revision_id IN (
  SELECT layer_revision_id
  FROM layer_revisions
  WHERE layer_id = 'water'
);

DELETE FROM layer_revisions
WHERE layer_id = 'water';

DELETE FROM layer_configs
WHERE layer_id = 'water';

DELETE FROM layers
WHERE layer_id = 'water';

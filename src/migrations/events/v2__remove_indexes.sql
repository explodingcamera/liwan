drop index events_event_idx on events (event);
drop index events_entity_id_idx on events (entity_id);
drop index events_visitor_id_idx on events (visitor_id);
drop index events_created_at_idx on events (created_at);
drop index events_entity_id_created_at_idx on events (entity_id, created_at);
drop index events_visitor_id_created_at_idx on events (visitor_id, created_at);

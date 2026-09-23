alter table events add column time_from_last_event interval;
alter table events add column time_to_next_event interval;

with cte as (
    select
        visitor_id,
        created_at,
        created_at - lag(created_at) over (partition by visitor_id order by created_at) as time_from_last_event,
        lead(created_at) over (partition by visitor_id order by created_at) - created_at as time_to_next_event
    from events
)
update events
set
    time_from_last_event = cte.time_from_last_event,
    time_to_next_event = cte.time_to_next_event
from cte
where events.visitor_id = cte.visitor_id and events.created_at = cte.created_at;
from rx.subject import Subject
class EventQueue:
    queue = Subject()
event_queue = EventQueue().queue
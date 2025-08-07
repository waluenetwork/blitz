use std::sync::{Arc, Mutex};
use tracing::debug;

use smithay::{
    backend::input::{
        Event, InputBackend, KeyboardKeyEvent, PointerButtonEvent, PointerMotionEvent,
        PointerAxisEvent, TouchDownEvent, TouchUpEvent, TouchMotionEvent,
        KeyState as SmithayKeyState, ButtonState, Axis, AxisSource,
    },
    input::{
        keyboard::{KeyboardHandle, KeysymHandle, ModifiersState},
        pointer::{PointerHandle, ButtonEvent, MotionEvent, AxisEvent as PointerAxisEvent},
        touch::{TouchHandle, DownEvent, UpEvent, MotionEvent as TouchMotionEvent},
    },
    utils::{Logical, Point},
};

use blitz_traits::events::{
    BlitzEvent, BlitzKeyEvent, BlitzMouseEvent, BlitzTouchEvent, BlitzScrollEvent,
    KeyState, MouseButton, TouchPhase,
};
use keyboard_types::{Code, Key, Location, Modifiers};

use crate::error::BlitzSmithayError;

pub struct WaylandEventHandler {
    event_queue: Arc<Mutex<Vec<BlitzEvent>>>,
    modifiers_state: ModifiersState,
}

impl WaylandEventHandler {
    pub fn new() -> Self {
        Self {
            event_queue: Arc::new(Mutex::new(Vec::new())),
            modifiers_state: ModifiersState::default(),
        }
    }
    
    pub fn handle_keyboard_event<B: InputBackend>(
        &mut self,
        event: &impl KeyboardKeyEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling keyboard event: key_code={:?} state={:?}", 
               event.key_code(), event.state());
        
        let blitz_event = self.smithay_keyboard_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Key(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_pointer_button_event<B: InputBackend>(
        &mut self,
        event: &impl PointerButtonEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling pointer button event: button={:?} state={:?}", 
               event.button_code(), event.state());
        
        let blitz_event = self.smithay_pointer_button_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Mouse(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_pointer_motion_event<B: InputBackend>(
        &mut self,
        event: &impl PointerMotionEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling pointer motion event: delta=({}, {})", 
               event.delta_x(), event.delta_y());
        
        let blitz_event = self.smithay_pointer_motion_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Mouse(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_pointer_axis_event<B: InputBackend>(
        &mut self,
        event: &impl PointerAxisEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling pointer axis event: axis={:?} value={}", 
               event.axis(), event.amount());
        
        let blitz_event = self.smithay_pointer_axis_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Scroll(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_touch_down_event<B: InputBackend>(
        &mut self,
        event: &impl TouchDownEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling touch down event: slot={:?} position=({}, {})", 
               event.slot(), event.x(), event.y());
        
        let blitz_event = self.smithay_touch_down_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Touch(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_touch_up_event<B: InputBackend>(
        &mut self,
        event: &impl TouchUpEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling touch up event: slot={:?}", event.slot());
        
        let blitz_event = self.smithay_touch_up_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Touch(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn handle_touch_motion_event<B: InputBackend>(
        &mut self,
        event: &impl TouchMotionEvent<B>,
    ) -> Result<(), BlitzSmithayError> {
        debug!("Handling touch motion event: slot={:?} position=({}, {})", 
               event.slot(), event.x(), event.y());
        
        let blitz_event = self.smithay_touch_motion_to_blitz(event)?;
        
        {
            let mut queue = self.event_queue
                .lock()
                .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
            queue.push(BlitzEvent::Touch(blitz_event));
        }
        
        Ok(())
    }
    
    pub fn drain_events(&mut self) -> Result<Vec<BlitzEvent>, BlitzSmithayError> {
        let mut queue = self.event_queue
            .lock()
            .map_err(|_| BlitzSmithayError::EventQueueLocked)?;
        Ok(queue.drain(..).collect())
    }
    
    fn smithay_keyboard_to_blitz<B: InputBackend>(
        &self,
        event: &impl KeyboardKeyEvent<B>,
    ) -> Result<BlitzKeyEvent, BlitzSmithayError> {
        let key_code = event.key_code();
        let state = match event.state() {
            SmithayKeyState::Pressed => KeyState::Pressed,
            SmithayKeyState::Released => KeyState::Released,
        };
        
        let code = self.smithay_keycode_to_blitz_code(key_code);
        let key = self.smithay_keycode_to_blitz_key(key_code);
        let modifiers = self.smithay_modifiers_to_blitz_modifiers();
        
        Ok(BlitzKeyEvent {
            key,
            code,
            modifiers,
            location: Location::Standard,
            is_auto_repeating: false,
            is_composing: false,
            state,
            text: None,
        })
    }
    
    fn smithay_pointer_button_to_blitz<B: InputBackend>(
        &self,
        event: &impl PointerButtonEvent<B>,
    ) -> Result<BlitzMouseEvent, BlitzSmithayError> {
        let button = self.smithay_button_to_blitz_button(event.button_code());
        let pressed = match event.state() {
            ButtonState::Pressed => true,
            ButtonState::Released => false,
        };
        
        Ok(BlitzMouseEvent {
            button,
            pressed,
            position: Point::from((0.0, 0.0)),
            modifiers: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_pointer_motion_to_blitz<B: InputBackend>(
        &self,
        event: &impl PointerMotionEvent<B>,
    ) -> Result<BlitzMouseEvent, BlitzSmithayError> {
        Ok(BlitzMouseEvent {
            button: MouseButton::None,
            pressed: false,
            position: Point::from((event.delta_x(), event.delta_y())),
            modifiers: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_pointer_axis_to_blitz<B: InputBackend>(
        &self,
        event: &impl PointerAxisEvent<B>,
    ) -> Result<BlitzScrollEvent, BlitzSmithayError> {
        let (delta_x, delta_y) = match event.axis() {
            Axis::Horizontal => (event.amount(), 0.0),
            Axis::Vertical => (0.0, event.amount()),
        };
        
        Ok(BlitzScrollEvent {
            delta_x,
            delta_y,
            position: Point::from((0.0, 0.0)),
            modifiers: self.smithay_modifiers_to_blitz_modifiers(),
        })
    }
    
    fn smithay_touch_down_to_blitz<B: InputBackend>(
        &self,
        event: &impl TouchDownEvent<B>,
    ) -> Result<BlitzTouchEvent, BlitzSmithayError> {
        Ok(BlitzTouchEvent {
            id: event.slot().into(),
            phase: TouchPhase::Started,
            position: Point::from((event.x(), event.y())),
            force: None,
        })
    }
    
    fn smithay_touch_up_to_blitz<B: InputBackend>(
        &self,
        event: &impl TouchUpEvent<B>,
    ) -> Result<BlitzTouchEvent, BlitzSmithayError> {
        Ok(BlitzTouchEvent {
            id: event.slot().into(),
            phase: TouchPhase::Ended,
            position: Point::from((0.0, 0.0)),
            force: None,
        })
    }
    
    fn smithay_touch_motion_to_blitz<B: InputBackend>(
        &self,
        event: &impl TouchMotionEvent<B>,
    ) -> Result<BlitzTouchEvent, BlitzSmithayError> {
        Ok(BlitzTouchEvent {
            id: event.slot().into(),
            phase: TouchPhase::Moved,
            position: Point::from((event.x(), event.y())),
            force: None,
        })
    }
    
    fn smithay_keycode_to_blitz_code(&self, keycode: u32) -> Code {
        match keycode {
            9 => Code::Escape,
            10 => Code::Digit1,
            11 => Code::Digit2,
            12 => Code::Digit3,
            13 => Code::Digit4,
            14 => Code::Digit5,
            15 => Code::Digit6,
            16 => Code::Digit7,
            17 => Code::Digit8,
            18 => Code::Digit9,
            19 => Code::Digit0,
            22 => Code::Backspace,
            23 => Code::Tab,
            24 => Code::KeyQ,
            25 => Code::KeyW,
            26 => Code::KeyE,
            27 => Code::KeyR,
            28 => Code::KeyT,
            29 => Code::KeyY,
            30 => Code::KeyU,
            31 => Code::KeyI,
            32 => Code::KeyO,
            33 => Code::KeyP,
            36 => Code::Enter,
            37 => Code::ControlLeft,
            38 => Code::KeyA,
            39 => Code::KeyS,
            40 => Code::KeyD,
            41 => Code::KeyF,
            42 => Code::KeyG,
            43 => Code::KeyH,
            44 => Code::KeyJ,
            45 => Code::KeyK,
            46 => Code::KeyL,
            50 => Code::ShiftLeft,
            52 => Code::KeyZ,
            53 => Code::KeyX,
            54 => Code::KeyC,
            55 => Code::KeyV,
            56 => Code::KeyB,
            57 => Code::KeyN,
            58 => Code::KeyM,
            62 => Code::ShiftRight,
            64 => Code::AltLeft,
            65 => Code::Space,
            _ => Code::Unidentified,
        }
    }
    
    fn smithay_keycode_to_blitz_key(&self, keycode: u32) -> Key {
        match keycode {
            9 => Key::Escape,
            10 => Key::Character("1".into()),
            11 => Key::Character("2".into()),
            12 => Key::Character("3".into()),
            13 => Key::Character("4".into()),
            14 => Key::Character("5".into()),
            15 => Key::Character("6".into()),
            16 => Key::Character("7".into()),
            17 => Key::Character("8".into()),
            18 => Key::Character("9".into()),
            19 => Key::Character("0".into()),
            22 => Key::Backspace,
            23 => Key::Tab,
            24 => Key::Character("q".into()),
            25 => Key::Character("w".into()),
            26 => Key::Character("e".into()),
            27 => Key::Character("r".into()),
            28 => Key::Character("t".into()),
            29 => Key::Character("y".into()),
            30 => Key::Character("u".into()),
            31 => Key::Character("i".into()),
            32 => Key::Character("o".into()),
            33 => Key::Character("p".into()),
            36 => Key::Enter,
            37 => Key::Control,
            38 => Key::Character("a".into()),
            39 => Key::Character("s".into()),
            40 => Key::Character("d".into()),
            41 => Key::Character("f".into()),
            42 => Key::Character("g".into()),
            43 => Key::Character("h".into()),
            44 => Key::Character("j".into()),
            45 => Key::Character("k".into()),
            46 => Key::Character("l".into()),
            50 => Key::Shift,
            52 => Key::Character("z".into()),
            53 => Key::Character("x".into()),
            54 => Key::Character("c".into()),
            55 => Key::Character("v".into()),
            56 => Key::Character("b".into()),
            57 => Key::Character("n".into()),
            58 => Key::Character("m".into()),
            62 => Key::Shift,
            64 => Key::Alt,
            65 => Key::Character(" ".into()),
            _ => Key::Unidentified,
        }
    }
    
    fn smithay_button_to_blitz_button(&self, button_code: u32) -> MouseButton {
        match button_code {
            0x110 => MouseButton::Left,
            0x111 => MouseButton::Right,
            0x112 => MouseButton::Middle,
            _ => MouseButton::Other(button_code as u16),
        }
    }
    
    fn smithay_modifiers_to_blitz_modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::default();
        
        if self.modifiers_state.ctrl {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.modifiers_state.alt {
            modifiers.insert(Modifiers::ALT);
        }
        if self.modifiers_state.shift {
            modifiers.insert(Modifiers::SHIFT);
        }
        if self.modifiers_state.logo {
            modifiers.insert(Modifiers::SUPER);
        }
        
        modifiers
    }
}

impl Default for WaylandEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

use super::*;

impl Controller<'_, '_> {
    fn recovery(&mut self, distance: i32, degree: i32) -> Option<u16> {
        if self.actor.status == 7 {
            return Some(self.succession(distance, degree));
        }
        if self.actor.status == 9 {
            return Some(0);
        }
        if self.actor.motion.action == MOT_ENGAGE {
            return Some(if self.random() % (self.engage() + 1) != 0 {
                DOWN
            } else {
                0
            });
        }
        None
    }

    fn reset_chase(&mut self) -> Result<()> {
        self.actor.brain.mode = 0;
        self.actor.brain.chase = [0; 2];
        self.item()
    }

    fn steer(&self, angle: i32) -> u16 {
        if self.degree() > angle {
            RIGHT
        } else if self.degree() < -angle {
            LEFT
        } else {
            0
        }
    }

    pub(super) fn short(&mut self) -> Result<u16> {
        if let Some(pad) = self.recovery(2000, 1000) {
            return Ok(pad);
        }
        let mut pad = 0;
        if self.actor.brain.mode == 0 {
            if self.distance() < 2500 && self.degree().abs() < 1500 {
                if self.actor.motion.count & 15 != 0 {
                    return Ok(0);
                }
                pad = self.steer(500) | SQUARE;
                self.actor.brain.mode = 1;
            } else {
                pad = self.chase(2000)?;
                if pad == 0 {
                    self.actor.brain.mode = 1;
                }
                if self.distance() > 4000 && self.degree().abs() < 100 && self.random() % 5 == 0 {
                    pad = UP | CROSS;
                }
                if self.attribute & ATTR_HIT != 0 {
                    self.actor.brain.mode = 1;
                }
            }
        } else if self.actor.motion.count & 15 != 0 {
            pad = self.actor.pad.held;
            if self.distance() < 1500 {
                if self.degree().abs() < 1000 {
                    pad = DOWN;
                } else if self.degree().abs() > 1500 {
                    pad = if self.distance() < 1000 {
                        self.command(1)
                    } else {
                        UP
                    };
                } else if self.random() % 30 == 0 {
                    pad = self.command(0x21);
                }
            }
        } else if self.distance() > 4000 {
            self.reset_chase()?;
            if self.distance() > 5000 {
                pad = UP | CROSS;
            }
        } else {
            if self.attribute & ATTR_WALL != 0 {
                self.actor.brain.mode = 0;
            }
            pad = self.steer(500);
            if self.distance() > 1500
                && self.distance() < 4000
                && self.degree().abs() < 1000
                && self.random() % (self.engage() + 1) == 0
                && self.clock > self.state.attack_clock
            {
                self.state.attack_clock = self.clock.wrapping_add(self.engage() * 10);
                if self.random() % 3 == 0 {
                    pad = DOWN;
                }
                return Ok(pad | SQUARE);
            }
            if self.degree().abs() > 1500 {
                pad |= DOWN;
            } else if self.distance() > 3000 {
                if self.degree().abs() < 200 && self.distance() > 3500 {
                    if self.random() & 1 != 0 {
                        pad = self.command(1);
                    } else if self.random() & 1 != 0 {
                        pad = self.command(0x21);
                    } else {
                        self.item()?;
                    }
                } else {
                    pad |= UP;
                }
            } else if self.distance() < 1500 {
                if self.degree() > 300 {
                    pad = self.command(3);
                } else if self.degree() < -300 {
                    pad = self.command(4);
                } else if self.distance() >= 1000 {
                    pad |= SQUARE;
                } else {
                    pad = if self.random() & 1 != 0 {
                        DOWN | CROSS
                    } else {
                        SQUARE | CIRCLE
                    };
                }
            } else if self.random() & 1 != 0 {
                pad = self.command(if self.degree() > 100 {
                    4
                } else if self.degree() < -100 {
                    3
                } else {
                    2
                });
            }
        }
        Ok(pad)
    }

    pub(super) fn general(&mut self) -> Result<u16> {
        if let Some(pad) = self.recovery(2000, 500) {
            return Ok(pad);
        }
        if self.actor.brain.mode == 0 {
            let mut pad = self.chase(3000)?;
            if pad == 0 || self.attribute & ATTR_HIT != 0 {
                self.actor.brain.mode = 1;
            }
            if self.distance() > 5000 && self.degree().abs() < 100 && self.random() % 5 == 0 {
                pad = UP | CROSS;
            }
            return Ok(pad);
        }
        if self.actor.motion.count & 15 != 0 {
            let mut pad = self.actor.pad.held;
            if self.distance() < 2000 {
                if self.degree().abs() < 1000 {
                    pad = DOWN;
                } else if self.degree().abs() > 1500 {
                    pad = UP;
                }
            }
            return Ok(pad);
        }
        if self.distance() > 5000 {
            self.reset_chase()?;
            return Ok(0);
        }
        if self.attribute & ATTR_WALL != 0 {
            self.actor.brain.mode = 0;
        }
        let mut pad = self.steer(500);
        if self.distance() > 1000
            && self.distance() < 3000
            && self.degree().abs() < 1200
            && self.random() % (self.engage() + 1) == 0
            && self.clock > self.state.attack_clock
        {
            self.state.attack_clock = self.clock.wrapping_add(self.engage() * 10);
            if self.random() % 3 == 0 {
                pad = DOWN;
            }
            return Ok(pad | SQUARE);
        }
        if self.degree().abs() > 1000 || self.distance() < 2000 {
            if self.distance() < 1000 {
                pad = match self.random() % 4 {
                    0 => DOWN | CROSS,
                    1 => SQUARE | CIRCLE,
                    2 => self.command(2),
                    _ => pad | SQUARE,
                };
            } else {
                pad |= DOWN;
            }
            return Ok(pad);
        }
        if self.distance() > 3000 {
            pad |= UP;
            if self.distance() > 4000 {
                if self.random() & 1 != 0 {
                    pad = self.command(1);
                } else if self.degree().abs() < 500 {
                    pad = self.command(0x21);
                } else {
                    self.item()?;
                }
            }
            return Ok(pad);
        }
        if self.random() & 1 != 0 {
            if self.degree() > 100 {
                pad = self.command(4);
            } else if self.degree() < -100 {
                pad = self.command(3);
            }
        }
        Ok(pad)
    }

    pub(super) fn long(&mut self) -> Result<u16> {
        if let Some(pad) = self.recovery(3000, 1500) {
            return Ok(pad);
        }
        if self.actor.brain.mode == 0 {
            let mut pad = self.chase(3000)?;
            if pad == 0 || self.attribute & ATTR_HIT != 0 {
                self.actor.brain.mode = 1;
            }
            if self.actor.motion.count == 0 && self.random() % 5 == 0 {
                pad = UP | CROSS;
            }
            return Ok(pad);
        }
        if self.actor.motion.count & 15 != 0 {
            let mut pad = self.actor.pad.held;
            if self.distance() < 3000 {
                if self.degree().abs() < 500 {
                    pad = DOWN;
                } else if self.degree().abs() > 1500 {
                    pad = UP;
                }
            }
            return Ok(pad);
        }
        if self.distance() > 5000 {
            self.reset_chase()?;
            return Ok(0);
        }
        if self.attribute & ATTR_WALL != 0 {
            self.actor.brain.mode = 0;
        }
        let mut pad = self.steer(300);
        if self.distance() > 3000
            && self.distance() < 4000
            && self.random() % (self.engage() + 1) == 0
        {
            self.state.attack_clock = self.clock.wrapping_add(self.engage() * 10);
            return Ok(pad | SQUARE);
        }
        if self.degree().abs() > 1000 || self.distance() < 3000 {
            if self.distance() < 1000 {
                pad = if self.random() & 1 != 0 {
                    DOWN | CROSS
                } else {
                    SQUARE | CIRCLE
                };
            } else {
                pad = SQUARE | if self.random() & 1 != 0 { LEFT } else { RIGHT };
            }
            return Ok(pad);
        }
        if self.distance() <= 4000 {
            return Ok(pad);
        }
        if self.degree().abs() < 50 {
            pad = self.command(0x21);
        } else if self.actor.motion.count != 0 {
            pad |= UP;
        } else if self.degree() > 50 {
            pad = self.command(4);
        } else if self.degree() < -50 {
            pad = self.command(3);
        } else if self.random() & 1 != 0 {
            pad = if self.random() & 1 != 0 {
                self.command(1)
            } else {
                SQUARE | CROSS
            };
        } else {
            self.item()?;
        }
        Ok(pad)
    }
}

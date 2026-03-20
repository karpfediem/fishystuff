use crate::bridge::contract::FishyMapInputState;
use crate::plugins::bookmarks::BookmarkState;

pub(super) fn apply_bookmarks(input: &FishyMapInputState, bookmarks: &mut BookmarkState) {
    if bookmarks.entries == input.ui.bookmarks {
        return;
    }
    bookmarks.entries = input.ui.bookmarks.clone();
}

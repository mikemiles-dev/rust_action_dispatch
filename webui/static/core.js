document.addEventListener('DOMContentLoaded', function() {
    // Select all elements with the 'utc-date' class
    const dateCells = document.querySelectorAll('.utc-date');

    dateCells.forEach(cell => {
        const timestamp = parseInt(cell.dataset.timestamp); // Get the timestamp from data-timestamp
        if (!isNaN(timestamp)) {
            const date = new Date(timestamp); // Create a Date object from the milliseconds timestamp

            // Format it as a UTC string
            // Option 1: Standard UTC string
            // const utcString = date.toUTCString();

            // Option 2: ISO 8601 UTC string (often preferred for consistency)
            // const utcString = date.toISOString();

            // Option 3: More readable custom UTC format using Intl.DateTimeFormat
            const options = {
                year: 'numeric',
                month: 'short',
                day: 'numeric',
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                hour12: false, // Use 24-hour format
                timeZone: 'UTC', // Ensure it's UTC
                timeZoneName: 'short' // e.g., "GMT" or "UTC"
            };
            const utcString = new Intl.DateTimeFormat('en-US', options).format(date);


            cell.textContent = utcString; // Replace the cell's content with the formatted date
        }
    });
});

function incrementPageAndReload() {
    const url = new URL(window.location.href);
    const currentPage = parseInt(url.searchParams.get('page')) || 1;
    url.searchParams.set('page', currentPage + 1);
    window.location.href = url.toString();
}

function decrementPageAndReload() {
    const url = new URL(window.location.href);
    const currentPage = parseInt(url.searchParams.get('page')) || 1;
    if (currentPage > 1) {
        url.searchParams.set('page', currentPage - 1);
        window.location.href = url.toString();
    }
}

function goToPage(pageNumber) {
    const url = new URL(window.location.href);
    url.searchParams.set('page', pageNumber);
    window.location = url.toString();
}

function applyFilterAndReload(filterName, filterValue) {
    const url = new URL(window.location.href);
    url.searchParams.set(filterName, filterValue);
    // Toggle 'order' query parameter between 'asc' and 'desc'
    const currentOrder = url.searchParams.get('order');
    if (currentOrder === 'asc') {
        url.searchParams.set('order', 'desc');
    } else if (currentOrder === 'desc') {
        url.searchParams.set('order', 'asc');
    }
    window.location.href = url.toString();
}

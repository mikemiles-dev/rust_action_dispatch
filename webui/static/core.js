// Global configuration for time format preference
window.prefer12HourFormat = true;

class DateTimeUtils {
    static formatUtcDate(timestamp) {
        if (isNaN(timestamp)) return '';
        const date = new Date(Number(timestamp));
        const options = {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: window.prefer12HourFormat || false,
            timeZone: 'UTC',
            timeZoneName: 'short'
        };
        return new Intl.DateTimeFormat('en-US', options).format(date);
    }

    static convertUtcDateElements() {
        document.querySelectorAll('.utc-date').forEach(cell => {
            const timestamp = cell.dataset.timestamp;
            cell.textContent = DateTimeUtils.formatUtcDate(timestamp);
        });
    }

    static setInputTime(elementId, utcEpochMs) {
        const url = new URL(window.location.href);
        if (!url.searchParams.has(elementId)) return;
        const date = new Date(utcEpochMs);
        const year = date.getUTCFullYear();
        const month = String(date.getUTCMonth() + 1).padStart(2, '0');
        const day = String(date.getUTCDate()).padStart(2, '0');
        const hours = String(date.getUTCHours()).padStart(2, '0');
        const minutes = String(date.getUTCMinutes()).padStart(2, '0');
        const formatted = `${year}-${month}-${day}T${hours}:${minutes}`;
        document.getElementById(elementId).value = formatted;
    }
}

class Pagination {
    static setPage(pageNumber) {
        const url = new URL(window.location.href);
        url.searchParams.set('page', pageNumber);
        window.location = url.toString();
    }

    static incrementPage() {
        const url = new URL(window.location.href);
        const currentPage = parseInt(url.searchParams.get('page')) || 1;
        url.searchParams.set('page', currentPage + 1);
        window.location.href = url.toString();
    }

    static decrementPage() {
        const url = new URL(window.location.href);
        const currentPage = parseInt(url.searchParams.get('page')) || 1;
        if (currentPage > 1) {
            url.searchParams.set('page', currentPage - 1);
            window.location.href = url.toString();
        }
    }
}

class FilterUtils {
    static applyFilterAndReload(filterName, filterValue, changeOrder = false, resetPage = false) {
        const url = new URL(window.location.href);
        url.searchParams.set(filterName, filterValue);

        // Handle range_start
        const rangeStartInput = document.getElementById('range_start');
        if (rangeStartInput && rangeStartInput.value.trim() !== '') {
            const startDate = new Date(rangeStartInput.value.trim());
            if (!isNaN(startDate.getTime())) {
                url.searchParams.set('range_start', startDate.getTime());
            }
        } else {
            url.searchParams.delete('range_start');
        }

        // Handle range_end
        const rangeEndInput = document.getElementById('range_end');
        if (rangeEndInput && rangeEndInput.value.trim() !== '') {
            const endDate = new Date(rangeEndInput.value.trim());
            if (!isNaN(endDate.getTime())) {
                url.searchParams.set('range_end', endDate.getTime());
            }
        } else {
            url.searchParams.delete('range_end');
        }

        if (resetPage) url.searchParams.set('page', 1);

        if (changeOrder) {
            const currentOrder = url.searchParams.get('order');
            url.searchParams.set('order', currentOrder === 'asc' ? 'desc' : 'asc');
        }

        window.location.href = url.toString();
    }
}

class AjaxUtils {
    static getJsonData(url, params = {}) {
        // Remove empty string values
        const filteredParams = Object.fromEntries(
            Object.entries(params).filter(([_, v]) => v !== '')
        );
        const currentUrlParams = Object.fromEntries(new URL(window.location.href).searchParams.entries());
        if (!('range_start' in currentUrlParams)) delete filteredParams['range_start'];
        if (!('range_end' in currentUrlParams)) delete filteredParams['range_end'];

        const queryString = Object.keys(filteredParams).length
            ? '?' + new URLSearchParams(filteredParams).toString()
            : '';
        const fullUrl = url + queryString;

        return fetch(fullUrl, {
            method: 'GET',
            headers: { 'Accept': 'application/json' }
        })
        .then(response => {
            if (!response.ok) throw new Error(`HTTP error! Status: ${response.status}`);
            return response.json();
        });
    }
}

// Usage examples (replace old function calls):
// DateTimeUtils.convertUtcDateElements();
// DateTimeUtils.setInputTime('myInput', 1710000000000);
// Pagination.incrementPage();
// Pagination.decrementPage();
// Pagination.setPage(3);
// FilterUtils.applyFilterAndReload('status', 'active', true, true);
// AjaxUtils.getJsonData('/api/data', { foo: 'bar' }).then(data => ...);

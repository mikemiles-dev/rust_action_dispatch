// Global configuration for time format preference
window.config = {
    _prefer12HourFormat: true,

    set prefer12HourFormat(value) {
        if (typeof value === 'boolean') {
            this._prefer12HourFormat = value;
        } else {
            console.warn('Invalid value for prefer12HourFormat. Must be a boolean.');
        }
    }
};

window.addEventListener('DOMContentLoaded', () => {
    const url = new URL(window.location.href);
    if (![...url.searchParams.keys()].length) {
        const savedParams = FilterUtils.getSavedUrlParamsFromSession();
        if (savedParams) {
            Object.entries(savedParams).forEach(([key, value]) => {
                url.searchParams.set(key, value);
            });
            window.location.replace(url.toString());
        }
    }
});

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
            hour12: window.config.prefer12HourFormat,
            timeZone: 'UTC',
            timeZoneName: 'short'
        };
        return new Intl.DateTimeFormat('en-US', options).format(date);
    }

    static convertUtcDateElements() {
        DateTimeUtils.utcDateElements = document.querySelectorAll('.utc-date');
        DateTimeUtils.utcDateElements.forEach(cell => {
            const timestamp = cell.dataset.timestamp;
            cell.textContent = DateTimeUtils.formatUtcDate(timestamp);
        });
    }

    static refreshUtcDateElementsCache() {
        DateTimeUtils.utcDateElements = document.querySelectorAll('.utc-date');
    }

    static setInputTime(elementId, utcEpochMs) {
        const url = new URL(window.location.href);
        if (!url.searchParams.has(elementId) || !utcEpochMs) return 0;
        const date = new Date(utcEpochMs);
        const options = {
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            hour12: window.config.prefer12HourFormat
        };
        // Format as "MM/DD/YYYY, hh:mm AM/PM" or "MM/DD/YYYY, HH:mm"
        const formattedDate = date.toLocaleString('en-US', { ...options, timeZone: 'UTC' }).replace(',', '');
        document.getElementById(elementId).value = formattedDate;
        return { date, formattedDate };
    }

    static parseUsDateTimeToIso(dateStringAsUtc) {
        // Manually parse the components
        const parts = dateStringAsUtc.match(/(\d{2})\/(\d{2})\/(\d{4}) (\d{1,2}):(\d{2}) (AM|PM)/);
        
        if (parts) {
            const month = parseInt(parts[1], 10) - 1; // Month is 0-indexed
            const day = parseInt(parts[2], 10);
            const year = parseInt(parts[3], 10);
            let hours = parseInt(parts[4], 10);
            const minutes = parseInt(parts[5], 10);
            const ampm = parts[6];
        
            // Adjust hours for AM/PM to 24-hour format
            if (ampm === "PM" && hours < 12) {
                hours += 12;
            }
            if (ampm === "AM" && hours === 12) { // 12 AM (midnight) is 0 hours
                hours = 0;
            }

            const utcDateObject = new Date(Date.UTC(year, month, day, hours, minutes, 0, 0));

            return utcDateObject;
        }
    }
}

class Pagination {
    // Updates the URL with the specified page number and navigates to the new URL.
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
        FilterUtils.setFilter(url, filterName, filterValue);
        FilterUtils.setDateTimeFilters(url);
        if (resetPage) FilterUtils.resetPage(url);
        if (changeOrder) FilterUtils.toggleOrder(url);
        FilterUtils.saveUrlParamsToSession();
        window.location.href = url.toString();
    }

    static getSavedUrlParamsFromSession() {
        const paramsJson = sessionStorage.getItem('savedUrlParams');
        if (!paramsJson) return null;
        try {
            return JSON.parse(paramsJson);
        } catch (e) {
            console.warn('Failed to parse savedUrlParams from sessionStorage:', e);
            return null;
        }
    }

    static saveUrlParamsToSession() {
        const url = new URL(window.location.href);
        const params = {};
        url.searchParams.forEach((value, key) => {
            params[key] = value;
        });
        sessionStorage.setItem('savedUrlParams', JSON.stringify(params));
    }

    static clearFilter() {
        FilterUtils.deleteSavedUrlParamsFromSession();
        window.location.href = window.location.pathname;
    }

    static deleteSavedUrlParamsFromSession() {
        sessionStorage.removeItem('savedUrlParams');
    }

    static deleteItemsFromDiv(baseUrl) {
        if (!window.confirm('Are you sure you want to delete these items?')) {
            return;
        }
        const div = document.getElementById('item_ids');
        if (!div) {
            console.warn('Div with id "item_ids" not found.');
            return;
        }
        const items = div.textContent.trim().split(/\s+/).filter(Boolean);
        fetch(baseUrl, {
            method: 'DELETE',
            headers: { 'Content-Type': 'application/json' },
            params: JSON.stringify({ items })
        })
        .then(response => {
            if (!response.ok) throw new Error('Failed to delete items');
            return response.json();
        })
        .then(data => {
            console.log('Delete response:', data);
        })
        .catch(error => {
            console.error('Error deleting items:', error);
        });
        window.location.href = baseUrl;
    }

    static setFilter(url, filterName, filterValue) {
        url.searchParams.set(filterName, filterValue);
    }

    static setDateTimeFilters(url) {
        FilterUtils.handleRangeInput(url, 'range_start');
        FilterUtils.handleRangeInput(url, 'range_end');
    }

    static handleRangeInput(url, rangeKey) {
        const rangeInput = document.getElementById(rangeKey);
        if (rangeInput && rangeInput.value.trim() !== '') {
            const utcDateTimeString = rangeInput.value.trim();
            const utcDate = DateTimeUtils.parseUsDateTimeToIso(utcDateTimeString);
            let epochMs = utcDate.getTime();
            url.searchParams.set(rangeKey, epochMs);
        } else {
            url.searchParams.delete(rangeKey);
        }
    }

    static resetPage(url) {
        url.searchParams.set('page', 1);
    }

    static toggleOrder(url) {
        const currentOrder = url.searchParams.get('order');
        url.searchParams.set('order', currentOrder === 'asc' ? 'desc' : 'asc');
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
            method: 'GET'
        })
        .then(response => {
            if (!response.ok) throw new Error(`HTTP error! Status: ${response.status}, URL: ${fullUrl}`);
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
